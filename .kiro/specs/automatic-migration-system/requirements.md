# 要件定義書

## 概要

アプリケーション起動時に、実行されていないマイグレーションを自動で適用するシステムを実装します。`migrations`テーブルを作成し、そこに適用済みのマイグレーションを記録していき、適用されていないマイグレーションを判別できるようにします。

## 用語集

- **Migration_System**: マイグレーション管理システム
- **Migration_Table**: 適用済みマイグレーションを記録するテーブル
- **Migration_Record**: 個別のマイグレーション実行記録
- **Application_Startup**: アプリケーション起動プロセス
- **Database_Connection**: データベース接続
- **Migration_Status**: マイグレーションの実行状態

## 要件

### 要件 1: マイグレーション記録テーブルの管理

**ユーザーストーリー:** システム管理者として、適用済みのマイグレーションを追跡したいので、マイグレーション記録テーブルが自動で管理されるようにしたい。

#### 受け入れ基準

1. WHEN アプリケーションが起動する THEN Migration_System SHALL `migrations`テーブルが存在しない場合は作成する
2. WHEN `migrations`テーブルを作成する THEN Migration_System SHALL 必要なカラム（id, name, applied_at, checksum）を含める
3. WHEN `migrations`テーブルを作成する THEN Migration_System SHALL 適切なインデックスを作成する
4. WHEN `migrations`テーブルが既に存在する THEN Migration_System SHALL テーブル構造を変更せずに処理を継続する

### 要件 2: マイグレーション実行状態の判定

**ユーザーストーリー:** 開発者として、どのマイグレーションが実行済みかを知りたいので、システムが自動で判定できるようにしたい。

#### 受け入れ基準

1. WHEN マイグレーション状態をチェックする THEN Migration_System SHALL 利用可能なマイグレーション一覧を取得する
2. WHEN マイグレーション状態をチェックする THEN Migration_System SHALL `migrations`テーブルから適用済みマイグレーション一覧を取得する
3. WHEN 利用可能なマイグレーションと適用済みマイグレーションを比較する THEN Migration_System SHALL 未適用のマイグレーション一覧を返す
4. WHEN マイグレーション名が重複している THEN Migration_System SHALL エラーを返す

### 要件 3: アプリケーション起動時の自動マイグレーション実行

**ユーザーストーリー:** ユーザーとして、アプリケーションを起動したときに必要なマイグレーションが自動で適用されるようにしたい。

#### 受け入れ基準

1. WHEN アプリケーションが起動する THEN Migration_System SHALL データベース接続確立後にマイグレーションチェックを実行する
2. WHEN 未適用のマイグレーションが存在する THEN Migration_System SHALL 各マイグレーションを順次実行する
3. WHEN マイグレーションを実行する THEN Migration_System SHALL 実行前にバックアップを作成する
4. WHEN マイグレーションが成功する THEN Migration_System SHALL `migrations`テーブルに実行記録を追加する
5. WHEN マイグレーションが失敗する THEN Migration_System SHALL エラーログを出力し、アプリケーション起動を停止する

### 要件 4: マイグレーション実行記録の管理

**ユーザーストーリー:** システム管理者として、いつどのマイグレーションが実行されたかを追跡したいので、詳細な実行記録が保存されるようにしたい。

#### 受け入れ基準

1. WHEN マイグレーションを実行する THEN Migration_System SHALL マイグレーション名、実行日時、チェックサムを記録する
2. WHEN マイグレーション記録を保存する THEN Migration_System SHALL 日本標準時（JST）で実行日時を記録する
3. WHEN マイグレーション記録を保存する THEN Migration_System SHALL マイグレーション内容のチェックサムを計算して保存する
4. WHEN 同じマイグレーション名で異なるチェックサムが検出される THEN Migration_System SHALL エラーを返す

### 要件 5: 既存マイグレーション機能との統合

**ユーザーストーリー:** 開発者として、既存のマイグレーション機能を活用したいので、新しいシステムが既存機能と統合されるようにしたい。

#### 受け入れ基準

1. WHEN 自動マイグレーションシステムを初期化する THEN Migration_System SHALL 既存の`run_migrations`関数を呼び出す
2. WHEN ユーザー認証マイグレーションが未適用の場合 THEN Migration_System SHALL `migrate_user_authentication`関数を呼び出す
3. WHEN receipt_urlマイグレーションが未適用の場合 THEN Migration_System SHALL `migrate_receipt_path_to_url`関数を呼び出す
4. WHEN 既存マイグレーションが完了している THEN Migration_System SHALL `migrations`テーブルに適切な記録を追加する

### 要件 6: エラーハンドリングとロールバック

**ユーザーストーリー:** システム管理者として、マイグレーション失敗時にデータが破損しないようにしたいので、適切なエラーハンドリングとロールバック機能が提供されるようにしたい。

#### 受け入れ基準

1. WHEN マイグレーション実行中にエラーが発生する THEN Migration_System SHALL トランザクションをロールバックする
2. WHEN マイグレーション失敗時 THEN Migration_System SHALL 詳細なエラーメッセージをログに出力する
3. WHEN マイグレーション失敗時 THEN Migration_System SHALL バックアップファイルの場所を通知する
4. WHEN 致命的なマイグレーションエラーが発生する THEN Migration_System SHALL アプリケーション起動を停止する

### 要件 7: マイグレーション状態の確認機能

**ユーザーストーリー:** 開発者として、現在のマイグレーション状態を確認したいので、状態確認用のコマンドが提供されるようにしたい。

#### 受け入れ基準

1. WHEN マイグレーション状態確認コマンドを実行する THEN Migration_System SHALL 適用済みマイグレーション一覧を表示する
2. WHEN マイグレーション状態確認コマンドを実行する THEN Migration_System SHALL 未適用マイグレーション一覧を表示する
3. WHEN マイグレーション状態確認コマンドを実行する THEN Migration_System SHALL 各マイグレーションの実行日時を表示する
4. WHEN マイグレーション状態確認コマンドを実行する THEN Migration_System SHALL データベースの整合性状態を表示する

### 要件 8: パフォーマンスと安全性

**ユーザーストーリー:** ユーザーとして、アプリケーション起動時間が長くならないようにしたいので、マイグレーションチェックが効率的に実行されるようにしたい。

#### 受け入れ基準

1. WHEN マイグレーション状態をチェックする THEN Migration_System SHALL 必要最小限のデータベースクエリで判定を完了する
2. WHEN 適用済みマイグレーションが多数存在する THEN Migration_System SHALL インデックスを活用して高速に検索する
3. WHEN マイグレーション実行中 THEN Migration_System SHALL データベースロックを適切に管理する
4. WHEN 同時に複数のアプリケーションインスタンスが起動する THEN Migration_System SHALL 重複実行を防止する