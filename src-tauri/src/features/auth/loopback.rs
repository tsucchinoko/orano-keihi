use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use url::Url;

/// OAuth認証コールバック情報
#[derive(Debug, Clone)]
pub struct OAuthCallback {
    /// 認証コード
    pub code: String,
    /// 状態パラメータ
    pub state: String,
    /// エラー情報（存在する場合）
    pub error: Option<String>,
}

/// ループバックHTTPサーバー
pub struct LoopbackServer {
    /// サーバーのポート番号
    port: u16,
    /// コールバック受信用のチャンネル
    callback_sender: Arc<Mutex<Option<oneshot::Sender<OAuthCallback>>>>,
}

impl LoopbackServer {
    /// 新しいループバックサーバーを作成する
    ///
    /// # 戻り値
    /// (LoopbackServer, ポート番号)
    pub fn new() -> Result<(Self, u16), Box<dyn std::error::Error + Send + Sync>> {
        // 利用可能なポートを自動で見つける
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener); // リスナーを閉じて、ポートを解放

        log::info!("ループバックサーバー用ポートを確保しました: {port}");

        let callback_sender = Arc::new(Mutex::new(None));

        let server = Self {
            port,
            callback_sender,
        };

        Ok((server, port))
    }

    /// サーバーを開始してOAuthコールバックを待機する
    ///
    /// # 戻り値
    /// OAuthコールバック情報を受信するReceiver
    pub async fn start_and_wait(
        &mut self,
    ) -> Result<oneshot::Receiver<OAuthCallback>, Box<dyn std::error::Error + Send + Sync>> {
        let (sender, receiver) = oneshot::channel();

        // senderをサーバーインスタンスに保存
        {
            let mut callback_sender = self.callback_sender.lock().unwrap();
            *callback_sender = Some(sender);
        }

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let callback_sender = Arc::clone(&self.callback_sender);

        // TCPリスナーを作成
        let listener = tokio::net::TcpListener::bind(addr).await?;
        log::info!("ループバックサーバーを開始しました: http://{addr}");

        // サーバーをバックグラウンドで実行
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let callback_sender = Arc::clone(&callback_sender);
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, callback_sender).await {
                                log::error!("接続処理エラー: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("接続受け入れエラー: {e}");
                        break;
                    }
                }
            }
        });

        Ok(receiver)
    }

    /// リダイレクトURIを取得する
    pub fn get_redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }
}

/// TCP接続を処理する
async fn handle_connection(
    stream: TcpStream,
    callback_sender: Arc<Mutex<Option<oneshot::Sender<OAuthCallback>>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let io = TokioIo::new(stream);

    let service = service_fn(move |req| handle_request(req, Arc::clone(&callback_sender)));

    if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
        log::error!("HTTP接続処理エラー: {err}");
    }

    Ok(())
}

/// HTTPリクエストを処理する
async fn handle_request(
    req: Request<Incoming>,
    callback_sender: Arc<Mutex<Option<oneshot::Sender<OAuthCallback>>>>,
) -> Result<Response<String>, Infallible> {
    log::debug!(
        "ループバックサーバーがリクエストを受信: {} {}",
        req.method(),
        req.uri()
    );

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/callback") => {
            // OAuth認証コールバックを処理
            let query = req.uri().query().unwrap_or("");
            let callback = parse_oauth_callback(query);

            log::info!(
                "OAuth認証コールバックを受信: code={}, state={}, error={:?}",
                callback.code.is_empty().then(|| "なし").unwrap_or("あり"),
                callback.state.is_empty().then(|| "なし").unwrap_or("あり"),
                callback.error
            );

            // コールバック情報を送信
            {
                let mut sender_guard = callback_sender.lock().unwrap();
                if let Some(sender) = sender_guard.take() {
                    if sender.send(callback).is_err() {
                        log::error!("コールバック情報の送信に失敗しました");
                    }
                }
            }

            // 成功レスポンスを返す
            let response_body = create_success_html();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html; charset=utf-8")
                .body(response_body)
                .unwrap())
        }
        _ => {
            // その他のリクエストは404を返す
            log::debug!("未対応のリクエスト: {} {}", req.method(), req.uri().path());
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Not Found".to_string())
                .unwrap())
        }
    }
}

/// OAuth認証コールバックのクエリパラメータを解析する
fn parse_oauth_callback(query: &str) -> OAuthCallback {
    let url = Url::parse(&format!("http://localhost/?{query}")).unwrap_or_else(|_| {
        log::warn!("無効なクエリパラメータ: {query}");
        Url::parse("http://localhost/").unwrap()
    });

    let mut code = String::new();
    let mut state = String::new();
    let mut error = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = value.to_string(),
            "state" => state = value.to_string(),
            "error" => error = Some(value.to_string()),
            _ => {}
        }
    }

    OAuthCallback { code, state, error }
}

/// 認証成功時のHTMLレスポンスを作成する
fn create_success_html() -> String {
    r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>認証完了 - オラの経費</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }
        .container {
            text-align: center;
            background: rgba(255, 255, 255, 0.1);
            padding: 2rem;
            border-radius: 1rem;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
        }
        .success-icon {
            font-size: 4rem;
            margin-bottom: 1rem;
        }
        h1 {
            margin: 0 0 1rem 0;
            font-size: 2rem;
        }
        p {
            margin: 0;
            font-size: 1.1rem;
            opacity: 0.9;
        }
        .close-instruction {
            margin-top: 1.5rem;
            font-size: 0.9rem;
            opacity: 0.7;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="success-icon">✅</div>
        <h1>認証が完了しました</h1>
        <p>Googleアカウントでのログインが成功しました。</p>
        <p class="close-instruction">このタブを閉じて、アプリケーションに戻ってください。</p>
    </div>
    <script>
        // 3秒後に自動でタブを閉じる
        setTimeout(() => {
            window.close();
        }, 3000);
    </script>
</body>
</html>"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_oauth_callback_success() {
        let query = "code=test_code&state=test_state";
        let callback = parse_oauth_callback(query);

        assert_eq!(callback.code, "test_code");
        assert_eq!(callback.state, "test_state");
        assert!(callback.error.is_none());
    }

    #[test]
    fn test_parse_oauth_callback_error() {
        let query = "error=access_denied&state=test_state";
        let callback = parse_oauth_callback(query);

        assert!(callback.code.is_empty());
        assert_eq!(callback.state, "test_state");
        assert_eq!(callback.error, Some("access_denied".to_string()));
    }

    #[test]
    fn test_create_success_html() {
        let html = create_success_html();
        assert!(html.contains("認証が完了しました"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[tokio::test]
    async fn test_loopback_server_creation() {
        let result = LoopbackServer::new();
        assert!(result.is_ok());

        let (server, port) = result.unwrap();
        assert!(port > 0);
        assert!(server.get_redirect_uri().contains(&port.to_string()));
    }
}
