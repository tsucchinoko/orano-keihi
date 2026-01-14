# 要件定義書

## はじめに

現在、ユーザーIDは連番（INTEGER AUTOINCREMENT）で管理されていますが、セキュリティとプライバシーの観点から、予測不可能なnanoIdに変更します。これにより、ユーザーIDの推測による不正アクセスのリスクを軽減し、より安全なシステムを構築します。

## 用語集

- **System**: ユーザー認証・管理システム全体
- **User_Repository**: ユーザーデータの永続化を担当するリポジトリ
- **Migration_Service**: データベーススキーマの変更を管理するサービス
- **NanoId**: 短くてURLセーフな一意識別子生成ライブラリ
- **Session_Manager**: セッション管理を担当するコンポーネント
- **Legacy_User_Id**: 既存の整数型ユーザーID
- **New_User_Id**: nanoIdベースの文字列型ユーザーID

## 要件

### 要件1: ユーザーIDの型変更

**ユーザーストーリー:** システム管理者として、ユーザーIDを予測不可能なnanoIdに変更したい。これにより、ユーザーIDの推測による不正アクセスを防止できる。

#### 受入基準

1. THE System SHALL usersテーブルのid列をINTEGERからTEXT型に変更する
2. WHEN 新規ユーザーが作成される THEN THE System SHALL 21文字のnanoIdを生成してユーザーIDとして割り当てる
3. THE System SHALL nanoIdの生成にURL-safeな文字セット（A-Za-z0-9_-）を使用する
4. WHEN ユーザーIDを生成する THEN THE System SHALL 衝突の可能性が極めて低いこと（1兆個のIDで1%未満の衝突確率）を保証する

### 要件2: 外部キー制約の更新

**ユーザーストーリー:** 開発者として、user_idを参照するすべてのテーブルの外部キー制約を更新したい。これにより、データの整合性を維持できる。

#### 受入基準

1. THE System SHALL expensesテーブルのuser_id列をINTEGERからTEXT型に変更する
2. THE System SHALL subscriptionsテーブルのuser_id列をINTEGERからTEXT型に変更する
3. THE System SHALL sessionsテーブルのuser_id列をINTEGERからTEXT型に変更する
4. THE System SHALL receipt_cacheテーブルのuser_id列をINTEGERからTEXT型に変更する
5. THE System SHALL migration_logsテーブルのuser_id列をINTEGERからTEXT型に変更する
6. THE System SHALL security_audit_logsテーブルのuser_id列をINTEGERからTEXT型に変更する
7. THE System SHALL すべての外部キー制約が正しく機能することを保証する

### 要件3: データマイグレーション

**ユーザーストーリー:** システム管理者として、既存のユーザーデータを新しいID形式に移行したい。これにより、既存ユーザーのデータを失うことなくシステムを更新できる。

#### 受入基準

1. WHEN マイグレーションを実行する THEN THE Migration_Service SHALL 既存の各ユーザーに新しいnanoIdを生成して割り当てる
2. WHEN マイグレーションを実行する THEN THE Migration_Service SHALL 旧IDと新IDのマッピングテーブルを作成する
3. WHEN マイグレーションを実行する THEN THE Migration_Service SHALL すべての関連テーブルのuser_id参照を新しいIDに更新する
4. IF マイグレーション中にエラーが発生した THEN THE Migration_Service SHALL すべての変更をロールバックする
5. WHEN マイグレーションが完了した THEN THE Migration_Service SHALL マイグレーション結果をログに記録する

### 要件4: Rustコードの型変更

**ユーザーストーリー:** 開発者として、Rustコード内のuser_id型をi64からStringに変更したい。これにより、型安全性を保ちながらnanoIdを扱える。

#### 受入基準

1. THE System SHALL User構造体のid フィールドをi64からStringに変更する
2. THE System SHALL Session構造体のuser_idフィールドをi64からStringに変更する
3. THE System SHALL すべてのリポジトリメソッドのuser_idパラメータをi64からStringに変更する
4. THE System SHALL すべてのコマンドハンドラのuser_idパラメータをi64から&strまたはStringに変更する
5. WHEN コンパイルを実行する THEN THE System SHALL 型エラーが発生しないことを保証する

### 要件5: NanoIdライブラリの統合

**ユーザーストーリー:** 開発者として、nanoIdライブラリをプロジェクトに統合したい。これにより、標準的で信頼性の高いID生成機能を使用できる。

#### 受入基準

1. THE System SHALL `nanoid`クレートをCargo.tomlに追加する
2. THE System SHALL nanoId生成用のユーティリティ関数を作成する
3. WHEN nanoIdを生成する THEN THE System SHALL デフォルトで21文字の長さを使用する
4. THE System SHALL nanoId生成関数をUser_Repositoryから呼び出し可能にする

### 要件6: 後方互換性の維持

**ユーザーストーリー:** システム管理者として、マイグレーション期間中も既存のセッションが有効であることを保証したい。これにより、ユーザーに影響を与えずにシステムを更新できる。

#### 受入基準

1. WHEN マイグレーションを実行する THEN THE System SHALL 既存のアクティブセッションを新しいユーザーIDに関連付ける
2. IF 旧IDでのセッション検証が試みられた THEN THE System SHALL マッピングテーブルを使用して新IDに変換する
3. WHEN マイグレーション完了後 THEN THE System SHALL 旧IDでのアクセスを拒否する

### 要件7: テストの更新

**ユーザーストーリー:** 開発者として、すべての既存テストを新しいID形式に対応させたい。これにより、リグレッションを防止できる。

#### 受入基準

1. THE System SHALL すべてのユニットテストでi64のuser_idをStringに変更する
2. THE System SHALL テストデータ生成時にnanoIdを使用する
3. WHEN テストを実行する THEN THE System SHALL すべてのテストが成功することを保証する
4. THE System SHALL マイグレーション処理自体のテストを追加する

### 要件8: エラーハンドリング

**ユーザーストーリー:** 開発者として、nanoId生成やマイグレーション中のエラーを適切に処理したい。これにより、システムの安定性を保証できる。

#### 受入基準

1. IF nanoId生成に失敗した THEN THE System SHALL 適切なエラーメッセージを返す
2. IF マイグレーション中にデータベースエラーが発生した THEN THE System SHALL トランザクションをロールバックする
3. IF 外部キー制約違反が発生した THEN THE System SHALL 詳細なエラー情報をログに記録する
4. WHEN エラーが発生した THEN THE System SHALL ユーザーに分かりやすいエラーメッセージを表示する

### 要件9: パフォーマンスの維持

**ユーザーストーリー:** システム管理者として、ID型変更後もクエリパフォーマンスが低下しないことを保証したい。これにより、ユーザー体験を維持できる。

#### 受入基準

1. THE System SHALL usersテーブルのid列にインデックスを維持する
2. THE System SHALL 外部キー列にインデックスを維持する
3. WHEN ユーザー検索を実行する THEN THE System SHALL 応答時間が変更前と同等であることを保証する
4. THE System SHALL TEXT型のPRIMARY KEYが効率的に動作することを確認する

### 要件10: ドキュメントの更新

**ユーザーストーリー:** 開発者として、API仕様とデータベーススキーマのドキュメントを更新したい。これにより、チームメンバーが変更内容を理解できる。

#### 受入基準

1. THE System SHALL データベーススキーマドキュメントを更新してuser_idがTEXT型であることを記載する
2. THE System SHALL API仕様でuser_idがstring型であることを明記する
3. THE System SHALL マイグレーション手順をドキュメント化する
4. THE System SHALL nanoIdの特性（長さ、文字セット、衝突確率）をドキュメント化する
