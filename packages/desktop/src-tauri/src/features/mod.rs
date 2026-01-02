/// 機能別モジュール
///
/// このモジュールは、アプリケーションの機能を機能別に整理したモジュール群を提供します。
/// 各機能モジュールは、その機能に関連するすべてのコード（モデル、コマンド、データベース操作、サービス）
/// を含む自己完結型のユニットです。
// 機能モジュールの宣言
pub mod auth;
pub mod expenses;
pub mod migrations;
pub mod receipts;
pub mod security;
pub mod subscriptions;
