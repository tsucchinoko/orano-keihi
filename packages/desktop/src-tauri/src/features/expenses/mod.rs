/// 経費機能モジュール
///
/// このモジュールは経費管理に関連するすべての機能を提供します：
/// - 経費の作成、読み取り、更新、削除（CRUD操作）
/// - 経費データのバリデーション
/// - 月別・カテゴリ別の経費取得
/// - 領収書URLの管理
/// - 領収書キャッシュの管理
// サブモジュールの宣言
pub mod api_commands;
pub mod models;

// 公開インターフェース：外部から使用可能な型と関数をエクスポート

// モデル
pub use models::{CreateExpenseDto, Expense, ReceiptCache, UpdateExpenseDto};

// APIコマンド（API Server経由のTauriコマンドハンドラー）
pub use api_commands::{
    create_expense, delete_expense, delete_expense_receipt, get_expenses, update_expense,
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
    }
}
