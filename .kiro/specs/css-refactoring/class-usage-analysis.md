# カスタムクラス使用状況調査結果

## 調査日時
2024年11月28日

## 調査対象
すべてのSvelteコンポーネントファイル（.svelte）

## 調査結果サマリー

### 使用されているカスタムクラス

#### 1. コンポーネントクラス

| クラス名 | 使用箇所 | 使用回数 |
|---------|---------|---------|
| `.card` | 複数のコンポーネント | 多数 |
| `.btn` | 複数のコンポーネント | 多数 |
| `.btn-primary` | +page.svelte, expenses/+page.svelte | 3回 |
| `.btn-info` | ExpenseForm.svelte, ExpenseItem.svelte, MonthSelector.svelte, ReceiptViewer.svelte, SubscriptionList.svelte | 10回以上 |
| `.btn-success` | SubscriptionList.svelte | 1回 |
| `.input` | ExpenseForm.svelte, MonthSelector.svelte, SubscriptionForm.svelte | 多数 |

#### 2. グラデーションクラス

| クラス名 | 使用箇所 | 使用回数 |
|---------|---------|---------|
| `.gradient-primary` | +page.svelte, expenses/+page.svelte | 2回 |
| `.gradient-info` | +page.svelte | 1回 |
| `.gradient-success` | - | 0回（未使用） |
| `.gradient-warning` | - | 0回（未使用） |
| `.bg-gradient-light` | - | 0回（CSS変数として使用） |

#### 3. カテゴリカラークラス

| クラス名 | 使用箇所 | 使用回数 |
|---------|---------|---------|
| `.bg-category-transport` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.bg-category-meals` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.bg-category-communication` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.bg-category-supplies` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.bg-category-entertainment` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.bg-category-other` | ExpenseItem.svelte, CategoryFilter.svelte, SubscriptionList.svelte | 3回 |
| `.category-*` (テキストカラー) | - | 0回（未使用） |

## 詳細な使用箇所

### src/routes/+page.svelte
- **`.card`**: サマリーカード、サブスクリプションセクション
- **`.btn`**: 再読み込みボタン
- **`.btn-primary`**: 再読み込みボタン
- **`.gradient-primary`**: クイックアクションカード
- **`.gradient-info`**: クイックアクションカード
- **CSS変数**: `var(--gradient-primary)`, `var(--color-transport)`, `var(--color-meals)`, etc.

### src/routes/+layout.svelte
- **CSS変数**: `var(--bg-gradient-light)`, `var(--gradient-primary)`
- **注意**: クラスとしては使用されていないが、CSS変数として参照

### src/routes/expenses/+page.svelte
- **`.btn`**: 再読み込みボタン
- **`.btn-primary`**: 再読み込みボタン
- **`.gradient-primary`**: フローティングアクションボタン（FAB）
- **CSS変数**: `var(--gradient-primary)`

### src/features/expenses/components/ExpenseForm.svelte
- **`.card`**: フォームコンテナ
- **`.input`**: すべての入力フィールド（日付、金額、カテゴリ、説明）
- **`.btn`**: 保存ボタン、キャンセルボタン
- **`.btn-primary`**: 保存ボタン
- **`.btn-info`**: 領収書選択ボタン

### src/features/expenses/components/ExpenseList.svelte
- **`.card`**: サマリーカード、空状態カード

### src/features/expenses/components/ExpenseItem.svelte
- **`.card`**: 経費アイテムカード
- **`.btn`**: 編集ボタン、削除ボタン
- **`.btn-info`**: 編集ボタン
- **`.bg-category-*`**: カテゴリカラーバー（6種類すべて）

### src/features/expenses/components/CategoryFilter.svelte
- **`.card`**: フィルターカード
- **`.bg-category-*`**: カテゴリカラーインジケーター（6種類すべて）

### src/features/expenses/components/MonthSelector.svelte
- **`.card`**: 月選択カード
- **`.btn`**: 前月・次月ボタン
- **`.btn-info`**: 前月・次月ボタン
- **`.btn-primary`**: 今月に戻るボタン
- **`.input`**: 年・月のセレクトボックス

### src/features/subscriptions/components/SubscriptionForm.svelte
- **`.card`**: フォームコンテナ
- **`.input`**: すべての入力フィールド
- **`.btn`**: 保存ボタン、キャンセルボタン
- **`.btn-primary`**: 保存ボタン

### src/features/subscriptions/components/SubscriptionList.svelte
- **`.card`**: 月額合計カード、サブスクリプションアイテムカード、空状態カード
- **`.btn`**: 編集ボタン、停止/再開ボタン
- **`.btn-info`**: 編集ボタン
- **`.btn-success`**: 再開ボタン
- **`.bg-category-*`**: カテゴリカラーバー（6種類すべて）

### src/features/receipts/components/ReceiptViewer.svelte
- **`.btn`**: 閉じるボタン、ズームボタン
- **`.btn-info`**: ズームボタン

## CSS変数の使用状況

以下のCSS変数が`style`属性やCSS内で直接参照されています：

### グラデーション変数
- `var(--gradient-primary)`: +page.svelte, +layout.svelte, expenses/+page.svelte
- `var(--gradient-info)`: +page.svelte
- `var(--bg-gradient-light)`: +layout.svelte

### カテゴリカラー変数
- `var(--color-transport)`: +page.svelte
- `var(--color-meals)`: +page.svelte
- `var(--color-communication)`: +page.svelte
- `var(--color-supplies)`: +page.svelte
- `var(--color-entertainment)`: +page.svelte
- `var(--color-other)`: +page.svelte

## 未使用のクラス

以下のクラスは定義されているが、コンポーネントで使用されていません：

1. **`.gradient-success`**: 定義されているが未使用
2. **`.gradient-warning`**: 定義されているが未使用
3. **`.bg-gradient-dark`**: 定義されているが未使用
4. **`.category-transport`** (テキストカラー): 定義されているが未使用
5. **`.category-meals`** (テキストカラー): 定義されているが未使用
6. **`.category-communication`** (テキストカラー): 定義されているが未使用
7. **`.category-supplies`** (テキストカラー): 定義されているが未使用
8. **`.category-entertainment`** (テキストカラー): 定義されているが未使用
9. **`.category-other`** (テキストカラー): 定義されているが未使用
10. **`.btn-warning`**: 定義されているが未使用

## リファクタリング時の注意点

### 必須クラス（削除不可）
以下のクラスは複数箇所で使用されているため、必ず維持する必要があります：

1. **`.card`**: 最も頻繁に使用されるクラス
2. **`.btn`**: すべてのボタンで使用
3. **`.btn-primary`**: プライマリアクション用
4. **`.btn-info`**: 情報アクション用
5. **`.btn-success`**: 成功アクション用
6. **`.input`**: すべての入力フィールドで使用
7. **`.gradient-primary`**: FABやアクションカードで使用
8. **`.gradient-info`**: アクションカードで使用
9. **`.bg-category-*`** (6種類): カテゴリカラーバーで使用

### 削除可能なクラス
以下のクラスは現在使用されていないため、削除を検討できます：

1. `.gradient-success`
2. `.gradient-warning`
3. `.bg-gradient-dark`
4. `.category-*` (テキストカラー版、6種類すべて)
5. `.btn-warning`

### CSS変数の扱い
以下のCSS変数は直接参照されているため、維持する必要があります：

- `--gradient-primary`
- `--gradient-info`
- `--bg-gradient-light`
- `--color-transport`
- `--color-meals`
- `--color-communication`
- `--color-supplies`
- `--color-entertainment`
- `--color-other`

## 推奨事項

1. **未使用クラスの削除**: 上記の削除可能なクラスを削除してCSSを軽量化
2. **CSS変数の統合**: カテゴリカラーをTailwindの`@theme`に統合
3. **コンポーネントクラスの最適化**: `.card`, `.btn`, `.input`を`@layer components`で再定義
4. **グラデーションの統合**: 使用中のグラデーションのみを`@layer utilities`で定義
5. **互換性の維持**: 既存のクラス名を維持し、内部実装のみを変更

## 次のステップ

1. Tailwind CSS v4の`@theme`ディレクティブでカテゴリカラーを定義
2. `@layer components`で`.card`, `.btn`, `.input`を再実装
3. `@layer utilities`でグラデーションクラスを定義
4. グローバルスタイルを最小化
5. 視覚的検証を実施
