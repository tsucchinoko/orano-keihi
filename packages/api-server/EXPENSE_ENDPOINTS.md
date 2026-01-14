# 経費APIエンドポイント

## 概要

経費関連のAPIエンドポイントが実装されました。これらのエンドポイントは、ユーザーの経費データをD1データベースで管理します。

## 実装されたエンドポイント

### 1. POST /api/v1/expenses

経費を作成します。

**リクエスト:**

```json
{
  "date": "2024-01-15",
  "amount": 1500,
  "category": "食費",
  "description": "テスト経費"
}
```

**レスポンス (201 Created):**

```json
{
  "success": true,
  "expense": {
    "id": 1,
    "user_id": "user-id",
    "date": "2024-01-15",
    "amount": 1500,
    "category": "食費",
    "description": "テスト経費",
    "receipt_url": null,
    "created_at": "2024-01-15T10:00:00+09:00",
    "updated_at": "2024-01-15T10:00:00+09:00"
  },
  "timestamp": "2024-01-15T10:00:00+09:00"
}
```

### 2. GET /api/v1/expenses/:id

指定されたIDの経費を取得します。

**レスポンス (200 OK):**

```json
{
  "success": true,
  "expense": {
    "id": 1,
    "user_id": "user-id",
    "date": "2024-01-15",
    "amount": 1500,
    "category": "食費",
    "description": "テスト経費",
    "receipt_url": null,
    "created_at": "2024-01-15T10:00:00+09:00",
    "updated_at": "2024-01-15T10:00:00+09:00"
  },
  "timestamp": "2024-01-15T10:00:00+09:00"
}
```

### 3. GET /api/v1/expenses

経費一覧を取得します。月とカテゴリでフィルタリング可能です。

**クエリパラメータ:**

- `month` (オプション): YYYY-MM形式の月フィルター
- `category` (オプション): カテゴリフィルター

**例:**

- `/api/v1/expenses` - すべての経費
- `/api/v1/expenses?month=2024-01` - 2024年1月の経費
- `/api/v1/expenses?category=食費` - 食費カテゴリの経費
- `/api/v1/expenses?month=2024-01&category=食費` - 2024年1月の食費

**レスポンス (200 OK):**

```json
{
  "success": true,
  "expenses": [
    {
      "id": 1,
      "user_id": "user-id",
      "date": "2024-01-15",
      "amount": 1500,
      "category": "食費",
      "description": "テスト経費",
      "receipt_url": null,
      "created_at": "2024-01-15T10:00:00+09:00",
      "updated_at": "2024-01-15T10:00:00+09:00"
    }
  ],
  "count": 1,
  "filters": {
    "month": "2024-01",
    "category": null
  },
  "timestamp": "2024-01-15T10:00:00+09:00"
}
```

### 4. PUT /api/v1/expenses/:id

経費を更新します。

**リクエスト:**

```json
{
  "amount": 2000,
  "description": "更新されたテスト経費"
}
```

**レスポンス (200 OK):**

```json
{
  "success": true,
  "expense": {
    "id": 1,
    "user_id": "user-id",
    "date": "2024-01-15",
    "amount": 2000,
    "category": "食費",
    "description": "更新されたテスト経費",
    "receipt_url": null,
    "created_at": "2024-01-15T10:00:00+09:00",
    "updated_at": "2024-01-15T10:05:00+09:00"
  },
  "timestamp": "2024-01-15T10:05:00+09:00"
}
```

### 5. DELETE /api/v1/expenses/:id

経費を削除します。

**レスポンス (200 OK):**

```json
{
  "success": true,
  "message": "経費が正常に削除されました",
  "expenseId": 1,
  "timestamp": "2024-01-15T10:10:00+09:00"
}
```

## 認証

すべてのエンドポイントは認証が必要です。リクエストヘッダーに`Authorization: Bearer <token>`を含める必要があります。

## アクセス制御

各ユーザーは自分の経費データのみにアクセスできます。他のユーザーの経費にアクセスしようとすると、403エラーが返されます。

## エラーレスポンス

### 400 Bad Request - バリデーションエラー

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "日付はYYYY-MM-DD形式である必要があります",
    "category": "validation",
    "timestamp": "2024-01-15T10:00:00+09:00",
    "requestId": "request-id",
    "retryable": false
  }
}
```

### 401 Unauthorized - 認証エラー

```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "認証が必要です",
    "category": "authentication",
    "timestamp": "2024-01-15T10:00:00+09:00",
    "requestId": "request-id",
    "retryable": false
  }
}
```

### 403 Forbidden - アクセス拒否

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "このリソースにアクセスする権限がありません",
    "category": "authorization",
    "timestamp": "2024-01-15T10:00:00+09:00",
    "requestId": "request-id",
    "retryable": false
  }
}
```

### 404 Not Found - リソースが見つからない

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "経費が見つかりません",
    "category": "resource",
    "timestamp": "2024-01-15T10:00:00+09:00",
    "requestId": "request-id",
    "retryable": false
  }
}
```

## 実装ファイル

- `src/routes/expenses.ts` - 経費ルーター
- `src/repositories/expense-repository.ts` - 経費リポジトリ
- `src/types/d1-dtos.ts` - DTO型定義
- `src/app.ts` - アプリケーション統合

## テスト

開発環境でテストするには、Cloudflare Workersの開発サーバー（wrangler dev）を使用する必要があります。

```bash
# Wrangler開発サーバーを起動
wrangler dev

# 別のターミナルでテストを実行
pnpm tsx test-expense-api.ts
```

## 注意事項

1. すべてのエンドポイントは認証が必要です
2. ユーザーは自分の経費データのみにアクセスできます
3. 日付はYYYY-MM-DD形式で指定する必要があります
4. 領収書URLはHTTPSで始まる必要があります
5. 開発環境ではD1データベースのエミュレーションが必要です
