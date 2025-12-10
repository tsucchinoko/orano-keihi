// R2サービス関連のモジュール

pub mod r2_client;
pub mod config;
pub mod cache_manager;

// エラー型の定義
use thiserror::Error;

#[derive(Debug, Error)]
pub enum R2Error {
    #[error("R2接続に失敗しました: {0}")]
    ConnectionFailed(String),
    
    #[error("アップロードに失敗しました: {0}")]
    UploadFailed(String),
    
    #[error("ダウンロードに失敗しました: {0}")]
    DownloadFailed(String),
    
    #[error("ファイルが見つかりません: {0}")]
    FileNotFound(String),
    
    #[error("認証情報が無効です")]
    InvalidCredentials,
    
    #[error("ネットワークエラー: {0}")]
    NetworkError(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("設定の読み込みに失敗しました: {0}")]
    LoadFailed(String),
    
    #[error("アカウントIDが設定されていません")]
    MissingAccountId,
    
    #[error("アクセスキーが設定されていません")]
    MissingAccessKey,
    
    #[error("シークレットキーが設定されていません")]
    MissingSecretKey,
    
    #[error("バケット名が設定されていません")]
    MissingBucketName,
    
    #[error("環境変数エラー: {0}")]
    EnvVarError(#[from] std::env::VarError),
}