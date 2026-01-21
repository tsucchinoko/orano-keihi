# Googleログイン仕様書

## 概要

Goodmorn-inプロジェクトでは、Tauriデスクトップアプリケーションに**Google OAuth 2.0認証（PKCE対応）**を実装しています。Google認証情報（Client ID/Secret）はAPIサーバー側で一元管理され、デスクトップアプリ側には秘匿情報を保存しないセキュアな設計となっています。

**✅ 実装完了**: デスクトップアプリ側からGoogle認証情報を完全に削除し、APIサーバー経由の認証フローに移行しました。

## アーキテクチャ

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   デスクトップ    │     │   APIサーバー    │     │   Google OAuth  │
│   アプリ(Tauri)  │ ──▶ │   (Lambda)      │ ──▶ │   サーバー       │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │  1. ログイン開始       │                       │
        │ ─────────────────────▶│                       │
        │                       │                       │
        │  2. 認証URL返却        │                       │
        │ ◀─────────────────────│                       │
        │                       │                       │
        │  3. ブラウザで認証      │                       │
        │ ─────────────────────────────────────────────▶│
        │                       │                       │
        │  4. コールバック受信    │                       │
        │ ◀─────────────────────────────────────────────│
        │                       │                       │
        │  5. トークン交換       │  6. トークン取得       │
        │ ─────────────────────▶│ ─────────────────────▶│
        │                       │                       │
        │  7. JWT + ユーザー情報 │  8. トークン返却       │
        │ ◀─────────────────────│ ◀─────────────────────│
        │                       │                       │
```

## コード構造

### デスクトップアプリ（Rust + Svelte）

#### バックエンド（Rust）
`/packages/desktop/src-tauri/src/features/oauth/`

| ファイル | 役割 |
|---------|------|
| `commands.rs` | Tauriコマンド（Webからの呼び出しインターフェース） |
| `server.rs` | ローカルHTTPサーバー管理（コールバック受信用） |
| `config.rs` | OAuth設定管理（APIサーバーURLのみ） |
| `token_manager.rs` | トークンライフサイクル管理（保存・更新） |
| `api_client.rs` | APIサーバー通信クライアント |
| `types.rs` | データ型定義 |
| `callback_validator.rs` | コールバック検証（CSRF対策） |
| `token_validator.rs` | トークン形式検証 |
| `error.rs` | エラーハンドリング |

#### フロントエンド（Svelte + TypeScript）
`/packages/desktop/src/features/auth/`

| ファイル | 役割 |
|---------|------|
| `components/GoogleLogin.svelte` | ログインボタンコンポーネント |
| `services/authService.ts` | 認証ビジネスロジック層 |
| `stores/authStore.ts` | 状態管理（Svelte Stores） |
| `types.ts` | フロントエンド型定義 |

### APIサーバー（Lambda）
`/packages/lambda/src/desktop_auth_api/`

| ファイル | 役割 |
|---------|------|
| `main.rs` | Lambda ハンドラーエントリーポイント |
| `oauth.rs` | OAuth 2.0フロー実装（PKCE対応） |
| `handlers.rs` | HTTPハンドラー（各エンドポイント） |
| `security.rs` | セキュリティ管理（CORS、ヘッダー） |
| `jwt.rs` | JWT トークン管理 |
| `config.rs` | 環境変数から設定読み込み |
| `types.rs` | API リクエスト/レスポンス型 |
| `error.rs` | エラーハンドリング |

## 認証フロー

### 初回ログインフロー

1. **ユーザーがログインボタン押下**

2. **デスクトップアプリ: ローカルサーバー起動**
   - ランダムポート（8000-9000範囲）で起動
   - 127.0.0.1のみでリッスン

3. **デスクトップアプリ → APIサーバー: POST /auth/google/start**
   ```json
   {
     "redirect_uri": "http://127.0.0.1:8345/callback"
   }
   ```

4. **APIサーバー: PKCE パラメータ生成**
   - code_verifier: 128文字のランダム文字列
   - code_challenge: SHA256(code_verifier) → Base64 URL Encoding
   - state: 32文字のランダム文字列（CSRF対策）

5. **APIサーバー → デスクトップアプリ: 認証URL返却**
   ```json
   {
     "auth_url": "https://accounts.google.com/o/oauth2/v2/auth?...",
     "state": "xxxx",
     "code_verifier": "yyyy"
   }
   ```

6. **デスクトップアプリ: ブラウザで認証URL開く**
   - state、code_verifierをセキュアストレージに保存

7. **ユーザー: Googleアカウントで認証**

8. **Google → ブラウザ: コールバックリダイレクト**
   ```
   http://127.0.0.1:8345/callback?code=xxx&state=yyy
   ```

9. **デスクトップアプリ: コールバック受信**
   - state検証（セキュアストレージの値と比較）
   - ローカルサーバーを停止

10. **デスクトップアプリ → APIサーバー: POST /auth/google/callback**
    ```json
    {
      "code": "xxx",
      "state": "yyy",
      "code_verifier": "yyyy",
      "redirect_uri": "http://127.0.0.1:8345/callback"
    }
    ```

11. **APIサーバー: トークン交換・ユーザー情報取得**
    - Google OAuth 2.0トークンエンドポイントにPOST
    - Google People APIでユーザー情報取得
    - JWT生成（1時間有効）

12. **APIサーバー → デスクトップアプリ: 認証結果返却**
    ```json
    {
      "access_token": "JWT_TOKEN",
      "token_type": "Bearer",
      "expires_in": 3600,
      "user": {
        "id": "google_user_id",
        "email": "user@example.com",
        "name": "User Name",
        "picture": "https://..."
      }
    }
    ```

13. **デスクトップアプリ: UI更新**
    - ユーザー情報を状態管理に保存
    - ログイン状態を表示

### トークン自動更新フロー

アプリ起動時および5分間隔で実行:

1. `AuthService.ensureValidToken()` 呼び出し
2. トークン有効期限チェック
3. 期限切れの場合、リフレッシュトークンで更新
4. 新トークンをセキュアストレージに保存
5. UI更新

## APIエンドポイント

### POST /auth/google/start
認証フロー開始。認証URLとPKCEパラメータを生成。

**リクエスト:**
```json
{
  "redirect_uri": "http://127.0.0.1:{port}/callback"
}
```

**レスポンス:**
```json
{
  "auth_url": "https://accounts.google.com/o/oauth2/v2/auth?...",
  "state": "random_state_string",
  "code_verifier": "random_code_verifier"
}
```

### POST /auth/google/callback
認証コードをトークンに交換。

**リクエスト:**
```json
{
  "code": "authorization_code",
  "state": "state_from_auth_start",
  "code_verifier": "code_verifier_from_auth_start",
  "redirect_uri": "http://127.0.0.1:{port}/callback"
}
```

**レスポンス:**
```json
{
  "access_token": "jwt_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "google_user_id",
    "email": "user@example.com",
    "name": "User Name",
    "picture": "https://..."
  }
}
```

### GET /config/public
公開設定取得（認証不要）

### GET /config/protected
保護設定取得（JWT認証必須）

## セキュリティ実装

### PKCE（Proof Key for Code Exchange）
認証コード横取り攻撃を防止。

```rust
// code_verifier: 128文字のランダム文字列
let code_verifier = generate_code_verifier();

// code_challenge: SHA256(code_verifier) → Base64 URL Encoding
let code_challenge = generate_code_challenge(&code_verifier);
```

### CSRF対策（Stateパラメータ）
```rust
// 認証URL生成時にランダムなstateを生成
let state = generate_random_string(32);

// コールバック受信時に検証
if received_state != stored_state {
    return Err(SecurityViolation);
}
```

### セキュアストレージ
- OS標準のキーリング（macOS Keychain等）使用
- トークンは暗号化して保存
- データベース接続情報は保存しない

### HTTPS強制
```rust
if !base_url.starts_with("https://") {
    return Err("HTTPSが必須です".to_string());
}
```

### コールバックサーバー
- ランダムポート（8000-9000）を自動選択
- 認証完了後、即座にサーバー停止
- 127.0.0.1のみでリッスン

### セキュリティヘッダー
```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Content-Security-Policy: default-src 'self'
```

## データモデル

### UserProfile
```rust
pub struct UserProfile {
    pub id: String,              // Google User ID
    pub email: String,
    pub name: String,
    pub picture: Option<String>, // プロフィール画像URL
    pub verified_email: bool,
    pub locale: Option<String>,  // ロケール（例: "ja"）
}
```

### LoginStatus
```rust
pub enum LoginStatus {
    NotLoggedIn,
    LoggingIn,
    LoggedIn,
    TokenExpired,
    Error(String),
}
```

### TokenInfo
```rust
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub token_type: String,
    pub scope: String,
}
```

## 環境変数

### デスクトップアプリ
```env
# APIサーバーURL（必須）
API_SERVER_URL=https://api.goodmorn-in.com

# セッション暗号化キー（32バイト以上推奨）
SESSION_ENCRYPTION_KEY=your_32_byte_random_encryption_key_here

# Google OAuth認証情報はAPIサーバー側で管理（デスクトップアプリには不要）
```

### APIサーバー
```env
# Google OAuth認証情報（必須）
GOOGLE_CLIENT_ID=877828004714-xxx.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=GOCSPX-xxx

# JWT署名用シークレット
JWT_SECRET=your-jwt-secret

# CORS設定
CORS_ORIGINS=tauri://localhost,https://app.example.com
```

## Slack認証との関係

### 従来のSlack認証アーキテクチャ
```
Slack Commands (/attendance)
  → 受付Lambda（Slack署名検証）
  → SQS キュー
  → 処理Lambda（Notion API更新）
  → Slack遅延レスポンス返信
```

### Google認証（新規追加）
```
Tauriデスクトップアプリ
  → APIサーバー経由でGoogle OAuth
  → JWT トークン発行
  → ユーザープロフィール取得
  → オフラインで勤怠記録作成
```

**主な違い:**
- SlackユーザーIDの代わりにGoogleアカウントIDで識別
- Slack統合は引き続き有効（Slackコマンドのみ）
- デスクトップアプリはGoogle認証のみ使用

## 設計上の特徴

### 1. クライアント側に秘匿情報なし
- Google OAuth Client ID/SecretはAPIサーバーのみが保持
- デスクトップアプリ側には`AUTH_API_URL`のみ設定
- バイナリ解析による情報漏洩リスクなし

### 2. APIサーバー経由の一元管理
- 認証フロー: デスクトップアプリ → APIサーバー → Google
- トークンはAPIサーバーで検証
- ユーザー情報はAPIサーバー経由でDB保存

### 3. 短期トークン設計
- APIサーバーが発行するJWTは有効期限1時間
- Google APIトークンはサーバー側で管理
- リフレッシュトークンのみクライアント保存

### 4. エラーハンドリング
- ユーザー向けメッセージ
- 技術的詳細
- 再試行可能判定
- 推奨アクション提示

### 5. ネットワーク監視
- 1分間隔でのネットワーク状態確認
- オフライン/オンライン切り替え
- キャッシュデータの自動同期

## 実装済み要件

- [x] ユーザーがGoogleアカウントでログイン可能
- [x] PKCE + stateパラメータによるセキュリティ実装
- [x] アプリ再起動時にセッション復元 + 自動トークン更新
- [x] ログイン状態とユーザー情報をUIに表示
- [x] OAuth設定がAPIサーバー側で管理
- [x] オフラインモード対応（キャッシュ使用）
- [x] 認証エラーの適切な処理
- [x] テストモード機能
- [x] ユーザー情報をAPIサーバー経由でデータベースに保存
- [x] データベース接続情報をクライアントに保存しない
- [x] **デスクトップアプリ側からGoogle認証情報を完全に削除**
- [x] **APIサーバー経由の認証フローに完全移行**

## 実装の変更点（2026年1月）

### デスクトップアプリ側の変更

1. **Google認証情報の削除**
   - `GOOGLE_CLIENT_ID`、`GOOGLE_CLIENT_SECRET`環境変数を削除
   - `.env.example`からGoogle OAuth設定を削除
   - `GoogleOAuthConfig`構造体を削除

2. **新しいAPIサービスの追加**
   - `ApiAuthService`を実装（`api_service.rs`）
   - APIサーバー経由でOAuth認証を実行
   - PKCE パラメータ（state、code_verifier）をAPIサーバーから取得

3. **環境変数の簡素化**
   ```env
   # 必要な環境変数（デスクトップアプリ）
   API_SERVER_URL=http://localhost:8787
   SESSION_ENCRYPTION_KEY=your_32_byte_random_encryption_key_here
   ```

### APIサーバー側の変更

1. **認証エンドポイントの追加**
   - `POST /api/v1/auth/google/start` - 認証フロー開始
   - `POST /api/v1/auth/google/callback` - 認証コールバック処理

2. **PKCE パラメータ生成**
   - `code_verifier`（128文字のランダム文字列）
   - `code_challenge`（SHA256ハッシュ + Base64 URL エンコード）
   - `state`（32文字のランダム文字列）

3. **JWT トークン発行**
   - Googleトークン交換後、独自のJWTトークンを発行
   - 有効期限: 1時間
   - ユーザー情報を含む

### セキュリティの向上

1. **秘匿情報の一元管理**
   - Google OAuth認証情報はAPIサーバーのみが保持
   - デスクトップアプリのバイナリ解析による情報漏洩リスクを排除

2. **PKCE フローの完全実装**
   - APIサーバーがPKCE パラメータを生成
   - デスクトップアプリは`code_verifier`を一時的に保持
   - 認証コールバック時にAPIサーバーに送信

3. **JWT トークンによる認証**
   - APIサーバーが発行するJWTトークンで認証
   - トークンの有効期限管理
   - リフレッシュトークンによる自動更新（今後実装予定）

### 移行手順

既存のデスクトップアプリを更新する場合:

1. **環境変数の更新**
   ```bash
   # packages/desktop/src-tauri/.env
   # 以下の行を削除
   # GOOGLE_CLIENT_ID=...
   # GOOGLE_CLIENT_SECRET=...
   # GOOGLE_REDIRECT_URI=...
   
   # 以下を追加（既にある場合は確認）
   API_SERVER_URL=http://localhost:8787
   SESSION_ENCRYPTION_KEY=your_32_byte_random_encryption_key_here
   ```

2. **APIサーバーの環境変数を設定**
   ```bash
   # packages/api-server/.env
   GOOGLE_CLIENT_ID=your_google_client_id_here
   GOOGLE_CLIENT_SECRET=your_google_client_secret_here
   JWT_SECRET=your_jwt_secret_here
   ```

3. **アプリケーションの再ビルド**
   ```bash
   cd packages/desktop
   pnpm tauri build
   ```

### 今後の改善予定

- [ ] リフレッシュトークンの実装
- [ ] トークン自動更新機能の強化
- [ ] オフライン時の認証状態管理の改善
- [ ] 複数デバイス間でのセッション同期
