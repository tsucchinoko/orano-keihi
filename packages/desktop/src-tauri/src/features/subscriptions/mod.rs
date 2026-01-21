/// サブスクリプション機能モジュール
///
/// このモジュールは、サブスクリプション管理に関連するすべての機能を提供します：
/// - サブスクリプションの作成、読み取り、更新、削除
/// - サブスクリプションの有効/無効切り替え
/// - 月額合計の計算
/// - 領収書パスの管理
/// - APIサーバー経由でのサブスクリプション操作
pub mod api_commands;
pub mod models;

// 公開インターフェース
pub use api_commands::{
    create_subscription, delete_subscription, delete_subscription_receipt_via_api,
    get_monthly_subscription_total, get_subscriptions, toggle_subscription_status,
    update_subscription,
};

pub use models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};
