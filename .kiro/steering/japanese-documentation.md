---
inclusion: always
---

# 日本語ドキュメンテーション規則

## 会話ルール

- ユーザーとの会話は常に日本語で行うこと
- タスク完了後の報告も日本語で行うこと
- エラーメッセージや説明も日本語で提供すること
- 技術的な説明も日本語で分かりやすく伝えること

## コーディング規則

### コメントとドキュメント
- すべてのコードコメントは日本語で記述すること
- 関数やモジュールの説明は日本語で記述すること
- 設計書、要件定義書、タスクリストなどのドキュメントは日本語で記述すること
- エラーメッセージやログメッセージは日本語で記述すること

### 例外
- 変数名、関数名、型名などの識別子は英語を使用すること（Rustの命名規則に従う）
- 外部ライブラリのAPIドキュメントは元の言語のまま
- コミットメッセージは英語でも日本語でも可

### コメントの書き方例

```rust
/// ユーザー情報を取得する
/// 
/// # 引数
/// * `user_id` - ユーザーID
/// 
/// # 戻り値
/// ユーザー情報、または見つからない場合はNone
pub fn get_user(user_id: i64) -> Option<User> {
    // データベースからユーザーを検索
    // ...
}
```

## 設計書の記述

- 要件定義書（requirements.md）は日本語で記述
- 設計書（design.md）は日本語で記述
- タスクリスト（tasks.md）は日本語で記述
- 技術的な用語は必要に応じて英語を併記（例：「データベース（Database）」）

## コマンド実行規則

### フロントエンド開発
- フロントエンド関連のコマンドは`pnpm`を使用すること
- 例：
  - `pnpm dev` - 開発サーバーの起動
  - `pnpm build` - プロダクションビルド
  - `pnpm check` - TypeScript型チェック
  - `pnpm fmt` - コードフォーマット
  - `pnpm lint` - コードリンティング

### バックエンド開発
- Rust/Tauri関連のコマンドは通常通り`cargo`を使用すること
- 例：`cargo build`, `cargo tauri dev`

### Tauriアプリケーション開発
- **Tauriアプリケーションの開発・テストには`pnpm tauri dev`を使用すること**
- フロントエンドのみのテストには`pnpm dev`を使用
- Tauriアプリケーション全体（フロントエンド + バックエンド）のテストには`pnpm tauri dev`を使用

## 日時の扱い

### タイムゾーン規則
- **すべての日時処理は日本標準時（JST / Asia/Tokyo）を使用すること**
- UTCを直接使用せず、必ずJSTに変換してから処理すること
- データベースへの保存時もJSTで記録すること

### Rust（バックエンド）での実装
```rust
use chrono::Utc;
use chrono_tz::Asia::Tokyo;

// 現在時刻をJSTで取得
let now_jst = Utc::now().with_timezone(&Tokyo);

// RFC3339形式で保存（JSTタイムゾーン付き）
let timestamp = now_jst.to_rfc3339();

// 日付の比較（今日の日付をJSTで取得）
let today = Utc::now().with_timezone(&Tokyo).date_naive();
```

### TypeScript（フロントエンド）での実装
```typescript
// 日付入力はYYYY-MM-DD形式で扱う
const date = "2024-11-28"; // input[type="date"]から取得

// バックエンドへの送信時はYYYY-MM-DD形式のまま送信
// ISO形式（toISOString()）は使用しない

// 今日の日付を取得（ローカル時刻）
const today = new Date().toISOString().split('T')[0];
```

### 注意事項
- `new Date().toISOString()`はUTCを返すため、日付の比較には使用しない
- 日付文字列（YYYY-MM-DD）の比較は文字列として直接比較可能
- タイムスタンプ生成時は必ずJSTを使用すること

## Svelte 5 リアクティブルール

### $state の使用規則
- **propsの初期値キャプチャを避けること**
- propsの値を$stateの初期値として直接使用しない
- 代わりに$effectブロック内でpropsの値を設定する

#### 悪い例
```typescript
let { expense }: Props = $props();
let amount = $state(expense?.amount.toString() || ""); // NG: 初期値のみキャプチャ
```

#### 良い例
```typescript
let { expense }: Props = $props();
let amount = $state("");

$effect(() => {
    // propsが変更されたときにリアクティブに更新
    if (expense) {
        amount = expense.amount.toString() || "";
    } else {
        amount = "";
    }
});
```

### $derived の使用規則
- **propsの値を使用する計算値は$derivedを使用すること**
- propsの値に依存する値は関数として定義する

#### 例
```typescript
let { selectedMonth }: Props = $props();

// propsの値を使用する計算値
const selectedYear = $derived(() => {
    const [year] = selectedMonth.split("-").map(Number);
    return year;
});

// 使用時は関数として呼び出す
console.log(selectedYear()); // selectedYear ではなく selectedYear()
```

### クラス属性でのリアクティブ値
- **クラス属性内でリアクティブ値を使用する場合は`class:`ディレクティブを使用すること**

#### 悪い例
```svelte
<button class="nav-link {isActive('/expenses') ? 'active' : ''}">
```

#### 良い例
```svelte
<button class="nav-link" class:active={isActive('/expenses')}>
```

### ナビゲーション
- **SvelteKitのクライアントサイドルーティングには`goto`関数を使用すること**
- 通常の`<a>`タグではなく、プログラム的ナビゲーションを推奨
- `data-sveltekit-preload-data`属性と組み合わせて使用

#### 例
```svelte
<script>
import { goto } from "$app/navigation";

function navigateTo(path: string) {
    goto(path);
}
</script>

<button onclick={() => navigateTo('/expenses')}>
    経費一覧
</button>
```
