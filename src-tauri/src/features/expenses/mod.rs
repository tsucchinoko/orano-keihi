/// 経費機能モジュール
///
/// このモジュールは経費管理に関連するすべての機能を提供します：
/// - 経費の作成、読み取り、更新、削除（CRUD操作）
/// - 経費データのバリデーション
/// - 月別・カテゴリ別の経費取得
/// - 領収書URLの管理
/// - 領収書キャッシュの管理
// サブモジュールの宣言
pub mod commands;
pub mod models;
pub mod repository;

// 公開インターフェース：外部から使用可能な型と関数をエクスポート

// モデル
pub use models::{CreateExpenseDto, Expense, ReceiptCache, UpdateExpenseDto};

// コマンド（Tauriコマンドハンドラー）
pub use commands::{create_expense, delete_expense, get_expenses, update_expense};

// リポジトリ（データベース操作）
pub use repository::{
    cleanup_old_cache, create, delete, delete_receipt_cache, find_all, find_by_id,
    get_receipt_cache, get_receipt_url, save_receipt_cache, set_receipt_url, update,
    update_cache_access_time,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // モジュールが正しくエクスポートされていることを確認

        // モデルのエクスポート確認
        let _expense: Option<Expense> = None;
        let _create_dto: Option<CreateExpenseDto> = None;
        let _update_dto: Option<UpdateExpenseDto> = None;
        let _receipt_cache: Option<ReceiptCache> = None;

        // この時点でコンパイルが通れば、エクスポートは正しく機能している
        assert!(true);
    }
}
