use thiserror::Error;

/// アプリケーション全体で使用される統一エラー型
#[derive(Debug, Error)]
pub enum AppError {
    /// データベース関連のエラー
    #[error("データベースエラー: {0}")]
    Database(String),

    /// バリデーション関連のエラー
    #[error("バリデーションエラー: {0}")]
    Validation(String),

    /// リソースが見つからない場合のエラー
    #[error("リソースが見つかりません: {0}")]
    NotFound(String),

    /// 外部サービス連携でのエラー
    #[error("外部サービスエラー: {0}")]
    ExternalService(String),

    /// セキュリティ関連のエラー
    #[error("セキュリティエラー: {0}")]
    Security(String),

    /// 設定関連のエラー
    #[error("設定エラー: {0}")]
    Configuration(String),

    /// I/O関連のエラー
    #[error("I/Oエラー: {0}")]
    Io(#[from] std::io::Error),

    /// JSON解析エラー
    #[error("JSON解析エラー: {0}")]
    Json(#[from] serde_json::Error),

    /// 並行処理関連のエラー
    #[error("並行処理エラー: {0}")]
    Concurrency(String),

    /// R2（AWS S3）関連のエラー
    #[error("R2エラー: {0}")]
    R2(String),
}

/// エラーの重要度を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorSeverity {
    /// 低重要度（ユーザー入力エラーなど）
    Low,
    /// 中重要度（外部サービス一時的エラーなど）
    Medium,
    /// 高重要度（データベースエラーなど）
    High,
    /// 最重要（セキュリティエラーなど）
    Critical,
}

impl AppError {
    /// ユーザーに表示するためのフレンドリーなメッセージを取得
    ///
    /// # 戻り値
    /// ユーザーに表示可能なエラーメッセージ
    pub fn user_message(&self) -> &str {
        match self {
            AppError::Database(_) => "データベース操作でエラーが発生しました",
            AppError::Validation(msg) => msg,
            AppError::NotFound(msg) => msg,
            AppError::ExternalService(_) => "外部サービスとの通信でエラーが発生しました",
            AppError::Security(_) => "セキュリティエラーが発生しました",
            AppError::Configuration(_) => "設定エラーが発生しました",
            AppError::Io(_) => "ファイル操作でエラーが発生しました",
            AppError::Json(_) => "データ形式の解析でエラーが発生しました",
            AppError::Concurrency(_) => "並行処理でエラーが発生しました",
            AppError::R2(_) => "クラウドストレージでエラーが発生しました",
        }
    }

    /// エラーの詳細情報を取得
    ///
    /// # 戻り値
    /// エラーの詳細情報（ログ出力用）
    pub fn details(&self) -> String {
        format!("{self}")
    }

    /// エラーの重要度を取得
    ///
    /// # 戻り値
    /// エラーの重要度レベル
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AppError::Database(_) => ErrorSeverity::High,
            AppError::Validation(_) => ErrorSeverity::Low,
            AppError::NotFound(_) => ErrorSeverity::Low,
            AppError::ExternalService(_) => ErrorSeverity::Medium,
            AppError::Security(_) => ErrorSeverity::Critical,
            AppError::Configuration(_) => ErrorSeverity::High,
            AppError::Io(_) => ErrorSeverity::Medium,
            AppError::Json(_) => ErrorSeverity::Medium,
            AppError::Concurrency(_) => ErrorSeverity::High,
            AppError::R2(_) => ErrorSeverity::Medium,
        }
    }

    /// バリデーションエラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `message` - バリデーションエラーメッセージ
    ///
    /// # 戻り値
    /// バリデーションエラー
    pub fn validation<S: Into<String>>(message: S) -> Self {
        AppError::Validation(message.into())
    }

    /// リソース未発見エラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `resource` - 見つからなかったリソース名
    ///
    /// # 戻り値
    /// リソース未発見エラー
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        AppError::NotFound(format!("{}が見つかりません", resource.into()))
    }

    /// 外部サービスエラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `service` - サービス名
    /// * `message` - エラーメッセージ
    ///
    /// # 戻り値
    /// 外部サービスエラー
    pub fn external_service<S: Into<String>>(service: S, message: S) -> Self {
        AppError::ExternalService(format!("{}: {}", service.into(), message.into()))
    }

    /// セキュリティエラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `message` - セキュリティエラーメッセージ
    ///
    /// # 戻り値
    /// セキュリティエラー
    pub fn security<S: Into<String>>(message: S) -> Self {
        AppError::Security(message.into())
    }

    /// 設定エラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `message` - 設定エラーメッセージ
    ///
    /// # 戻り値
    /// 設定エラー
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        AppError::Configuration(message.into())
    }

    /// 並行処理エラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `message` - 並行処理エラーメッセージ
    ///
    /// # 戻り値
    /// 並行処理エラー
    pub fn concurrency<S: Into<String>>(message: S) -> Self {
        AppError::Concurrency(message.into())
    }

    /// R2エラーを作成するヘルパー関数
    ///
    /// # 引数
    /// * `message` - R2エラーメッセージ
    ///
    /// # 戻り値
    /// R2エラー
    pub fn r2<S: Into<String>>(message: S) -> Self {
        AppError::R2(message.into())
    }
}

/// AppErrorからStringへの変換（Tauriコマンドでの使用のため）
impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.user_message().to_string()
    }
}

/// rusqlite::ErrorからAppErrorへの変換
impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::Database(error.to_string())
    }
}

/// Result型のエイリアス（アプリケーション全体で使用）
pub type AppResult<T> = Result<T, AppError>;

/// R2Error型のエイリアス（後方互換性のため）
pub type R2Error = AppError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        // 各エラータイプの重要度をテスト
        assert_eq!(
            AppError::validation("テスト").severity(),
            ErrorSeverity::Low
        );
        assert_eq!(
            AppError::not_found("ユーザー").severity(),
            ErrorSeverity::Low
        );
        assert_eq!(
            AppError::external_service("R2", "接続失敗").severity(),
            ErrorSeverity::Medium
        );
        assert_eq!(
            AppError::security("不正アクセス").severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            AppError::configuration("設定ファイル不正").severity(),
            ErrorSeverity::High
        );
    }

    #[test]
    fn test_user_message() {
        // ユーザーメッセージのテスト
        let validation_error = AppError::validation("金額が不正です");
        assert_eq!(validation_error.user_message(), "金額が不正です");

        let not_found_error = AppError::not_found("経費");
        assert_eq!(not_found_error.user_message(), "経費が見つかりません");

        let security_error = AppError::security("認証失敗");
        assert_eq!(
            security_error.user_message(),
            "セキュリティエラーが発生しました"
        );
    }

    #[test]
    fn test_helper_functions() {
        // ヘルパー関数のテスト
        let validation_error = AppError::validation("テストメッセージ");
        assert!(matches!(validation_error, AppError::Validation(_)));

        let not_found_error = AppError::not_found("テストリソース");
        assert!(matches!(not_found_error, AppError::NotFound(_)));

        let external_error = AppError::external_service("TestService", "テストエラー");
        assert!(matches!(external_error, AppError::ExternalService(_)));
    }

    #[test]
    fn test_string_conversion() {
        // String変換のテスト
        let error = AppError::validation("テストエラー");
        let error_string: String = error.into();
        assert_eq!(error_string, "テストエラー");
    }

    #[test]
    fn test_error_details() {
        // エラー詳細のテスト
        let error = AppError::validation("詳細テスト");
        let details = error.details();
        assert!(details.contains("詳細テスト"));
    }
}
