# 設計書

## 概要

本設計では、現在の`src/app.css`に定義されている多数のカスタムCSSクラスを、Tailwind CSS v4の機能を活用してリファクタリングします。主な目標は以下の通りです：

1. CSS変数とカスタムカラーをTailwindのテーマシステムに統合
2. 共通コンポーネントクラス（`.card`, `.btn`, `.input`）を`@layer components`で定義
3. グローバルスタイルを最小限に抑える
4. 既存のコンポーネントとの互換性を維持

Tailwind CSS v4では、設定ファイルが不要になり、CSS内で直接`@theme`ディレクティブを使用してカスタマイズできます。これにより、より直感的で保守しやすいスタイルシステムを構築できます。

## アーキテクチャ

### レイヤー構造

Tailwind CSSの3つのレイヤーを活用します：

1. **@layer base**: 最小限のグローバルスタイル（body、リセットなど）
2. **@layer components**: 再利用可能なコンポーネントクラス（`.card`, `.btn`, `.input`）
3. **@layer utilities**: カスタムユーティリティクラス（グラデーション、カテゴリカラー）

### ファイル構成

```
src/
  app.css          # メインCSSファイル（リファクタリング対象）
  features/
    expenses/
      components/  # 既存コンポーネント（変更なし）
  routes/
    +page.svelte   # 既存ページ（変更なし）
```

## コンポーネントとインターフェース

### 1. テーマ定義（@theme）

Tailwind CSS v4の`@theme`ディレクティブを使用して、カスタムカラーを定義します。

```css
@theme {
  /* カテゴリカラー */
  --color-category-transport: #3b82f6;
  --color-category-meals: #ef4444;
  --color-category-communication: #8b5cf6;
  --color-category-supplies: #10b981;
  --color-category-entertainment: #f59e0b;
  --color-category-other: #6b7280;
}
```

これにより、`bg-category-transport`, `text-category-meals`のような標準的なTailwindクラスが自動生成されます。

### 2. CSS変数（グラデーション）

グラデーションは複雑なため、CSS変数として保持し、カスタムユーティリティクラスから参照します。

```css
:root {
  --gradient-primary: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  --gradient-success: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  --gradient-info: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  --gradient-warning: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
  --bg-gradient-light: linear-gradient(to bottom right, #fafafa, #e5e7eb);
}
```

### 3. コンポーネントクラス（@layer components）

共通コンポーネントを`@layer components`で定義します。これにより、Tailwindユーティリティで簡単に上書きできます。

```css
@layer components {
  .card {
    @apply bg-white rounded-xl shadow-md p-6 transition-shadow duration-200;
  }
  
  .card:hover {
    @apply shadow-lg;
  }
  
  .btn {
    @apply px-4 py-2 rounded-lg font-semibold cursor-pointer border-0 text-white transition-all duration-200;
  }
  
  .btn:hover {
    @apply -translate-y-0.5 shadow-md;
  }
  
  .btn:active {
    @apply translate-y-0;
  }
  
  .input {
    @apply px-3 py-2 border-2 border-gray-200 rounded-lg w-full transition-all duration-200;
  }
  
  .input:focus {
    @apply outline-none ring-2 ring-purple-500 border-transparent;
  }
}
```

### 4. ユーティリティクラス（@layer utilities）

グラデーション用のカスタムユーティリティクラスを定義します。

```css
@layer utilities {
  .gradient-primary {
    background: var(--gradient-primary);
  }
  
  .gradient-success {
    background: var(--gradient-success);
  }
  
  .gradient-info {
    background: var(--gradient-info);
  }
  
  .gradient-warning {
    background: var(--gradient-warning);
  }
  
  .bg-gradient-light {
    background: var(--bg-gradient-light);
  }
}
```

### 5. ベーススタイル（@layer base）

最小限のグローバルスタイルのみを定義します。

```css
@layer base {
  body {
    @apply min-h-screen bg-gradient-light;
    font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
}
```

## データモデル

本機能はスタイルのリファクタリングであり、データモデルの変更はありません。

## 正確性プロパティ

*プロパティとは、システムのすべての有効な実行において真であるべき特性や動作のことです。本質的には、システムが何をすべきかについての形式的な記述です。プロパティは、人間が読める仕様と機械で検証可能な正確性の保証との橋渡しとなります。*

本機能はスタイルのリファクタリングであり、従来のプロパティベーステストは適用できません。代わりに、以下の検証可能な特性を定義します。これらは手動テストまたは視覚的回帰テストツールで検証します。

### プロパティ1: クラス名の互換性

*すべての*既存コンポーネントで使用されているカスタムクラス名（`.card`, `.btn`, `.btn-primary`, `.btn-success`, `.btn-info`, `.btn-warning`, `.input`, `.gradient-*`, `.bg-category-*`, `.category-*`）が、リファクタリング後のCSSファイルで定義されていること

**検証: 要件 4.1, 4.3**

### プロパティ2: Tailwindユーティリティによる上書き可能性

*すべての*コンポーネントクラス（`.card`, `.btn`, `.input`）において、Tailwindユーティリティクラス（例：`bg-blue-500`, `p-8`, `text-lg`）を追加した場合、そのユーティリティのスタイルが優先されて適用されること

**検証: 要件 2.4, 4.4**

## エラーハンドリング

本機能はスタイルのリファクタリングであり、実行時エラーは発生しません。ただし、以下の点に注意が必要です：

1. **CSS構文エラー**: `@theme`, `@layer`, `@apply`の構文が正しいことを確認
2. **クラス名の不一致**: 既存コンポーネントで使用されているクラス名がすべて定義されていることを確認
3. **ビルドエラー**: Tailwind CSS v4のビルドプロセスが正常に動作することを確認

## テスト戦略

### ユニットテスト

本機能はスタイルのリファクタリングであり、従来のユニットテストは適用できません。代わりに、以下の手動テストを実施します：

1. **視覚的回帰テスト**: リファクタリング前後でスクリーンショットを比較
2. **ブラウザ検証**: 各ページを開いて、スタイルが正しく適用されていることを確認
3. **レスポンシブテスト**: 異なる画面サイズでレイアウトが崩れていないことを確認

### 検証手順

1. **ビルドの成功**: `deno task build`が正常に完了すること
2. **開発サーバーの起動**: `deno task dev`でエラーが発生しないこと
3. **ページの表示**: すべてのページが正しく表示されること
4. **インタラクション**: ホバー、フォーカス、アクティブ状態が正しく動作すること

### テストケース

| テストケース | 検証内容 | 期待結果 |
|------------|---------|---------|
| カードコンポーネント | `.card`クラスのスタイル | 白背景、角丸、影が適用される |
| ボタンコンポーネント | `.btn`, `.btn-primary`のスタイル | グラデーション背景、ホバー効果が動作 |
| 入力フィールド | `.input`クラスのスタイル | ボーダー、フォーカス時のリング効果 |
| カテゴリカラー | `bg-category-transport`などのクラス | 正しい色が適用される |
| グラデーション | `.gradient-primary`などのクラス | グラデーションが表示される |
| レスポンシブ | モバイル画面でのレイアウト | カードのパディングが調整される |

## 実装の詳細

### Tailwind CSS v4の特徴

1. **設定ファイル不要**: `tailwind.config.js`が不要になり、CSS内で直接カスタマイズ
2. **@themeディレクティブ**: カスタムカラー、フォント、スペーシングなどを定義
3. **パフォーマンス向上**: Oxide（Rustベース）エンジンによる高速ビルド
4. **後方互換性**: 既存のTailwindクラスはすべて動作

### 移行戦略

1. **段階的リファクタリング**: 一度にすべてを変更せず、セクションごとに移行
2. **互換性の維持**: 既存のクラス名を保持し、内部実装のみを変更
3. **検証**: 各セクションの移行後に視覚的検証を実施

### パフォーマンスへの影響

- **CSSファイルサイズ**: カスタムCSSが減少し、Tailwindユーティリティに統合されるため、最終的なCSSサイズは変わらないか、わずかに減少
- **ビルド時間**: Tailwind CSS v4のOxideエンジンにより、ビルド時間は短縮される可能性
- **実行時パフォーマンス**: スタイルの適用方法は変わらないため、実行時パフォーマンスへの影響はなし
