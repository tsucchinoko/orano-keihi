/// サブスクリプション機能モジュール
///
/// このモジュールは、サブスクリプション管理に関連するすべての機能を提供します：
/// - サブスクリプションの作成、読み取り、更新、削除
/// - サブスクリプションの有効/無効切り替え
/// - 月額合計の計算
/// - 領収書パスの管理
pub mod commands;
pub mod models;
pub mod repository;

// 公開インターフェース
pub use commands::{
    create_subscription, get_monthly_subscription_total, get_subscriptions,
    toggle_subscription_status, update_subscription,
};

pub use models::{CreateSubscriptionDto, Subscription, UpdateSubscriptionDto};

pub use repository::{
    calculate_monthly_total, create, delete, find_all, find_by_id, get_receipt_path,
    set_receipt_path, toggle_status, update,
};
