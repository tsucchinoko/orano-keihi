# 実装計画: ユーザーIDのnanoId移行

## 概要

本実装計画は、ユーザーIDを連番（INTEGER）からnanoId（TEXT）に移行するための段階的な実装手順を定義します。各タスクは前のタスクの上に構築され、最終的にすべてのコンポーネントを統合します。

## タスク

- [ ] 1. NanoIdライブラリの統合とユーティリティ関数の作成
  - Cargo.tomlに`nanoid = "0.4.0"`を追加
  - `packages/desktop/src-tauri/src/shared/utils/nanoid.rs`を作成
  - `generate_user_id()`関数を実装（21文字のnanoIdを生成）
  - `generate_user_id_with_length(length: usize)`関数を実装（テスト用）
  - `is_valid_nanoid(id: &str) -> bool`関数を実装（バリデーション用）
  - `shared/utils/mod.rs`に`pub mod nanoid;`を追加
  - _要件: 5.1, 5.2, 5.3_

- [ ]* 1.1 NanoId生成関数のユニットテストを作成
  - `test_generate_user_id_length()` - 長さが21文字であることを確認
  - `test_generate_user_id_uniqueness()` - 2つのIDが異なることを確認
  - `test_generate_user_id_url_safe()` - URL-safe文字のみを含むことを確認
  - `test_generate_user_id_with_custom_length()` - カスタム長が機能することを確認
  - `test_is_valid_nanoid()` - バリデーション関数のテスト
  - _要件: 1.2, 1.3, 1.4_

- [ ]* 1.2 NanoId生成のプロパティベーステストを作成
  - **プロパティ1: NanoId長さの一貫性** - すべてのIDが21文字
  - **プロパティ2: NanoId文字セットの安全性** - すべてのIDがURL-safe文字のみ
  - **プロパティ3: NanoId一意性** - 異なる生成で異なるIDが生成される
  - _要件: 1.2, 1.3, 1.4_

- [ ] 2. データモデルの型変更
  - [ ] 2.1 User構造体のid型をi64からStringに変更
    - `packages/desktop/src-tauri/src/features/auth/models.rs`を更新
    - `User`構造体の`id: i64`を`id: String`に変更
    - _要件: 4.1_

  - [ ] 2.2 Session構造体のuser_id型をi64からStringに変更
    - `packages/desktop/src-tauri/src/features/auth/models.rs`を更新
    - `Session`構造体の`user_id: i64`を`user_id: String`に変更
    - _要件: 4.2_

  - [ ] 2.3 Expense関連のDTOのuser_id型を更新
    - `packages/desktop/src-tauri/src/features/expenses/models.rs`を更新
    - `CreateExpenseDto`の`user_id: Option<i64>`を`user_id: Option<String>`に変更
    - _要件: 4.3_

- [ ] 3. UserRepositoryの更新
  - [ ] 3.1 UserRepositoryのメソッドシグネチャを更新
    - `get_user_by_id(&self, user_id: &str)`に変更
    - `delete_user(&self, user_id: &str)`に変更
    - `get_user_by_id_internal(&self, conn: &Connection, user_id: &str)`に変更
    - _要件: 4.3_

  - [ ] 3.2 create_new_user関数でnanoIdを使用
    - `crate::shared::utils::nanoid::generate_user_id()`を呼び出してIDを生成
    - `INSERT`文で生成したIDを使用
    - `last_insert_rowid()`の代わりに生成したIDでユーザーを取得
    - _要件: 1.2, 5.4_

  - [ ] 3.3 row_to_user関数の型変換を更新
    - `id: row.get(0)?`がString型を返すことを確認
    - _要件: 4.1_

- [ ]* 3.4 UserRepositoryのテストを更新
  - すべてのテストでi64のuser_idをStringに変更
  - テストデータ生成時にnanoIdを使用
  - 既存のテストがすべて成功することを確認
  - _要件: 7.1, 7.2, 7.3_

- [ ] 4. SessionManagerの更新
  - `create_session(&self, user_id: &str)`のシグネチャを変更
  - `invalidate_user_sessions(&self, user_id: &str)`のシグネチャを変更
  - `get_session_internal`でuser_idの型変換を更新
  - _要件: 4.3_

- [ ]* 4.1 SessionManagerのテストを更新
  - すべてのテストでi64のuser_idをStringに変更
  - 既存のテストがすべて成功することを確認
  - _要件: 7.1, 7.3_

- [ ] 5. ExpenseRepositoryの更新
  - すべてのメソッドの`user_id: i64`パラメータを`user_id: &str`に変更
  - `create(conn: &Connection, dto: CreateExpenseDto, user_id: &str)`
  - `find_by_id(conn: &Connection, id: i64, user_id: &str)`
  - `find_all(conn: &Connection, user_id: &str, ...)`
  - `update(conn: &Connection, id: i64, dto: UpdateExpenseDto, user_id: &str)`
  - `delete(conn: &Connection, id: i64, user_id: &str)`
  - その他すべてのuser_idパラメータを持つ関数
  - _要件: 4.3_

- [ ]* 5.1 ExpenseRepositoryのテストを更新
  - すべてのテストでi64のuser_idをStringに変更
  - `DEFAULT_USER_ID`定数をString型に変更
  - 既存のテストがすべて成功することを確認
  - _要件: 7.1, 7.3_

- [ ] 6. SubscriptionRepositoryの更新
  - すべてのメソッドの`user_id: i64`パラメータを`user_id: &str`に変更
  - `create(conn: &Connection, dto: CreateSubscriptionDto, user_id: &str)`
  - `find_by_id(conn: &Connection, id: i64, user_id: &str)`
  - `find_all(conn: &Connection, user_id: &str, ...)`
  - その他すべてのuser_idパラメータを持つ関数
  - _要件: 4.3_

- [ ]* 6.1 SubscriptionRepositoryのテストを更新
  - すべてのテストでi64のuser_idをStringに変更
  - `DEFAULT_USER_ID`定数をString型に変更
  - 既存のテストがすべて成功することを確認
  - _要件: 7.1, 7.3_

- [ ] 7. UserPathManagerの更新
  - `generate_user_receipt_path(user_id: &str, expense_id: i64, filename: &str)`に変更
  - `convert_legacy_to_user_path(legacy_path: &str, user_id: &str)`に変更
  - `extract_user_id_from_path(path: &str) -> AppResult<String>`に変更（戻り値をStringに）
  - `validate_user_access(user_id: &str, path: &str)`に変更
  - `generate_user_subscription_path(user_id: &str, subscription_id: i64, filename: &str)`に変更
  - `convert_legacy_subscription_to_user_path(legacy_path: &str, user_id: &str)`に変更
  - `validate_admin_or_user_access(user_id: &str, is_admin: bool, path: &str)`に変更
  - 正規表現パターンを更新して文字列のuser_idを抽出
  - _要件: 4.3_

- [ ] 8. コマンドハンドラの更新
  - [ ] 8.1 Auth関連コマンドの更新
    - `packages/desktop/src-tauri/src/features/auth/commands.rs`を更新
    - すべてのコマンドハンドラでuser_idの型をStringに変更
    - _要件: 4.4_

  - [ ] 8.2 Expense関連コマンドの更新
    - `packages/desktop/src-tauri/src/features/expenses/commands.rs`を更新
    - すべてのコマンドハンドラでuser_idの型をStringに変更
    - _要件: 4.4_

  - [ ] 8.3 Subscription関連コマンドの更新
    - `packages/desktop/src-tauri/src/features/subscriptions/commands.rs`を更新
    - すべてのコマンドハンドラでuser_idの型をStringに変更
    - _要件: 4.4_

  - [ ] 8.4 Receipt関連コマンドの更新
    - `packages/desktop/src-tauri/src/features/receipts/commands.rs`を更新
    - すべてのコマンドハンドラでuser_idの型をStringに変更
    - _要件: 4.4_

- [ ] 9. チェックポイント - コンパイルエラーの確認
  - `cargo check`を実行してコンパイルエラーがないことを確認
  - 残っている型エラーがあれば修正
  - ユーザーに質問があれば確認

- [ ] 10. マイグレーション関数の実装
  - [ ] 10.1 メインマイグレーション関数を作成
    - `packages/desktop/src-tauri/src/features/migrations/service.rs`に`execute_user_id_nanoid_migration`関数を追加
    - 一時的なマッピングテーブル（user_id_mapping）を作成
    - 既存ユーザーに新しいnanoIdを割り当ててマッピングテーブルに保存
    - 新しいusersテーブル（users_new）を作成
    - データを移行
    - _要件: 3.1, 3.2_

  - [ ] 10.2 expensesテーブルのマイグレーション関数を実装
    - `migrate_expenses_table(tx: &Transaction)`関数を作成
    - 新しいスキーマでexpenses_newテーブルを作成（user_id TEXT型）
    - マッピングテーブルを使用してデータを移行
    - 旧テーブルを削除して新テーブルをリネーム
    - インデックスを再作成
    - _要件: 2.1, 3.3_

  - [ ] 10.3 subscriptionsテーブルのマイグレーション関数を実装
    - `migrate_subscriptions_table(tx: &Transaction)`関数を作成
    - 同様の処理をsubscriptionsテーブルに適用
    - _要件: 2.2, 3.3_

  - [ ] 10.4 sessionsテーブルのマイグレーション関数を実装
    - `migrate_sessions_table(tx: &Transaction)`関数を作成
    - 同様の処理をsessionsテーブルに適用
    - _要件: 2.3, 3.3, 6.1_

  - [ ] 10.5 receipt_cacheテーブルのマイグレーション関数を実装
    - `migrate_receipt_cache_table(tx: &Transaction)`関数を作成
    - 同様の処理をreceipt_cacheテーブルに適用
    - _要件: 2.4, 3.3_

  - [ ] 10.6 migration_logsテーブルのマイグレーション関数を実装
    - `migrate_migration_logs_table(tx: &Transaction)`関数を作成
    - 同様の処理をmigration_logsテーブルに適用
    - _要件: 2.5, 3.3_

  - [ ] 10.7 security_audit_logsテーブルのマイグレーション関数を実装
    - `migrate_security_audit_logs_table(tx: &Transaction)`関数を作成
    - 同様の処理をsecurity_audit_logsテーブルに適用
    - _要件: 2.6, 3.3_

  - [ ] 10.8 マイグレーション関数を統合
    - `execute_user_id_nanoid_migration`から各テーブルのマイグレーション関数を呼び出し
    - 旧usersテーブルを削除して新テーブルをリネーム
    - インデックスを再作成
    - マッピングテーブルを削除
    - _要件: 1.1, 2.7, 3.3_

  - [ ] 10.9 マイグレーションをマイグレーションレジストリに登録
    - `get_all_migrations()`関数に新しいマイグレーションを追加
    - マイグレーション番号とバージョンを設定
    - _要件: 3.5_

- [ ]* 10.10 マイグレーション関数のユニットテストを作成
  - `test_migration_creates_mapping_table()` - マッピングテーブルが作成されることを確認
  - `test_migration_converts_user_ids()` - ユーザーIDが変換されることを確認
  - `test_migration_preserves_user_count()` - ユーザー数が保持されることを確認
  - `test_migration_updates_foreign_keys()` - 外部キーが更新されることを確認
  - _要件: 3.1, 3.3, 7.4_

- [ ]* 10.11 マイグレーションのプロパティベーステストを作成
  - **プロパティ5: マイグレーション後のユーザーID形式** - すべてのユーザーIDがnanoId形式
  - **プロパティ6: マイグレーション後のデータ整合性** - 関連テーブルのuser_id参照が正しく更新
  - **プロパティ8: セッション移行の正確性** - セッションのuser_idが正しく更新
  - _要件: 3.1, 3.3, 6.1_

- [ ] 11. エラーハンドリングの実装
  - [ ] 11.1 マイグレーションエラーハンドリングを追加
    - `execute_migration`関数でトランザクションを使用
    - エラー時に自動ロールバック
    - 詳細なエラーログを記録
    - _要件: 3.4, 8.2_

  - [ ] 11.2 user_idバリデーションを追加
    - `get_user_by_id`などでuser_idがnanoId形式であることを検証
    - 無効な形式の場合は`AuthError::InvalidUserId`を返す
    - _要件: 8.1_

  - [ ] 11.3 外部キー制約違反のエラーハンドリングを追加
    - 制約違反時に詳細なエラー情報をログに記録
    - ユーザーに分かりやすいエラーメッセージを返す
    - _要件: 8.3_

- [ ]* 11.4 エラーハンドリングのプロパティベーステストを作成
  - **プロパティ7: マイグレーションエラー時のロールバック** - エラー時にすべての変更がロールバック
  - _要件: 3.4, 8.2_

- [ ] 12. チェックポイント - マイグレーションのテスト実行
  - テスト用データベースでマイグレーションを実行
  - すべてのテーブルが正しく移行されることを確認
  - すべてのテストが成功することを確認
  - ユーザーに質問があれば確認

- [ ] 13. 統合とワイヤリング
  - [ ] 13.1 アプリケーション起動時のマイグレーション実行
    - `packages/desktop/src-tauri/src/main.rs`または初期化コードでマイグレーションを実行
    - マイグレーションが既に実行済みかチェック
    - _要件: 3.5_

  - [ ] 13.2 すべてのコンポーネントが新しいString型のuser_idで動作することを確認
    - 認証フロー全体をテスト
    - 経費作成・取得・更新・削除をテスト
    - サブスクリプション作成・取得・更新・削除をテスト
    - _要件: 4.5_

- [ ]* 13.3 外部キー制約のプロパティベーステストを作成
  - **プロパティ4: 外部キー制約の機能性** - ユーザー削除時に関連データがカスケード削除
  - _要件: 2.7_

- [ ]* 13.4 統合テストを作成
  - エンドツーエンドのユーザー作成・認証・データ操作フローをテスト
  - マイグレーション後のシステム全体の動作をテスト
  - _要件: 4.5_

- [ ] 14. 最終チェックポイント - すべてのテストが成功することを確認
  - `cargo test`を実行してすべてのテストが成功することを確認
  - `cargo clippy`を実行してリントエラーがないことを確認
  - ユーザーに質問があれば確認

## 注意事項

- `*`マークが付いたタスクはオプションであり、コア機能の実装を優先する場合はスキップ可能です
- 各タスクは特定の要件を参照しており、トレーサビリティを確保しています
- チェックポイントタスクは段階的な検証を保証します
- プロパティベーステストは普遍的な正確性プロパティを検証します
- ユニットテストは特定の例とエッジケースを検証します
