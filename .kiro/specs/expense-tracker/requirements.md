# Requirements Document

## Introduction

確定申告のための経費精算を効率化する、サブスクリプション料金、領収書、日々の支出（カフェ代、飲み会代など）を管理するデスクトップアプリケーション。SvelteKitとTauriを使用し、SQLiteをデータベースとして採用。モダンでおしゃれなグラデーションUIを特徴とする。

## Glossary

- **Expense Tracker**: 経費を記録・管理するデスクトップアプリケーション
- **Expense Entry**: カフェ代や飲み会代などの個別の支出記録
- **Subscription**: 月払いまたは年払いの定期支払い
- **Receipt**: 領収書の画像またはPDFファイル
- **Category**: 経費の分類（交通費、飲食費、通信費など）
- **SQLite Database**: ローカルに保存されるデータベース
- **User**: アプリケーションを使用する個人事業主またはフリーランサー

## Requirements

### Requirement 1

**User Story:** As a User, I want to record individual expenses with date, amount, category, and description, so that I can track my daily spending for tax purposes

#### Acceptance Criteria

1. THE Expense Tracker SHALL provide a form to input expense date, amount, category, and description
2. WHEN the User submits a valid expense entry, THE Expense Tracker SHALL store the entry in the SQLite Database
3. THE Expense Tracker SHALL validate that the amount is a positive number before saving
4. THE Expense Tracker SHALL validate that the date is not in the future before saving
5. WHEN the User saves an expense entry, THE Expense Tracker SHALL display a confirmation message

### Requirement 2

**User Story:** As a User, I want to attach receipt images or PDFs to expense entries, so that I can keep digital copies of my receipts for tax documentation

#### Acceptance Criteria

1. THE Expense Tracker SHALL allow the User to attach image files (PNG, JPG, JPEG) or PDF files to an expense entry
2. WHEN the User attaches a receipt file, THE Expense Tracker SHALL store the file path in the SQLite Database
3. THE Expense Tracker SHALL display a thumbnail preview of attached receipt images
4. WHEN the User clicks on a receipt thumbnail, THE Expense Tracker SHALL open the full-size receipt in a modal view
5. THE Expense Tracker SHALL limit receipt file size to 10 megabytes per file

### Requirement 3

**User Story:** As a User, I want to manage recurring subscription payments, so that I can track monthly and annual subscription costs automatically

#### Acceptance Criteria

1. THE Expense Tracker SHALL provide a form to create subscription entries with name, amount, billing cycle (monthly or annual), start date, and category
2. WHEN the User creates a subscription, THE Expense Tracker SHALL store the subscription details in the SQLite Database
3. THE Expense Tracker SHALL display all active subscriptions in a dedicated subscription list view
4. THE Expense Tracker SHALL allow the User to mark a subscription as inactive
5. THE Expense Tracker SHALL calculate the total monthly subscription cost across all active subscriptions

### Requirement 4

**User Story:** As a User, I want to view my expenses organized by month and category, so that I can analyze my spending patterns for tax preparation

#### Acceptance Criteria

1. THE Expense Tracker SHALL display expenses grouped by month in a list view
2. THE Expense Tracker SHALL display the total amount spent per category for the selected month
3. WHEN the User selects a different month, THE Expense Tracker SHALL update the expense list and category totals
4. THE Expense Tracker SHALL display expenses in descending order by date (newest first)
5. THE Expense Tracker SHALL show the grand total of all expenses for the selected month

### Requirement 5

**User Story:** As a User, I want to categorize my expenses using predefined categories, so that I can organize expenses according to tax deduction categories

#### Acceptance Criteria

1. THE Expense Tracker SHALL provide predefined categories including "交通費" (Transportation), "飲食費" (Meals), "通信費" (Communication), "消耗品費" (Supplies), "接待交際費" (Entertainment), and "その他" (Other)
2. WHEN creating or editing an expense or subscription, THE Expense Tracker SHALL display a dropdown list of available categories
3. THE Expense Tracker SHALL require the User to select a category before saving an expense or subscription
4. THE Expense Tracker SHALL allow filtering expenses by category in the list view
5. THE Expense Tracker SHALL display category-specific color coding in the expense list

### Requirement 6

**User Story:** As a User, I want to edit or delete existing expense entries, so that I can correct mistakes or remove duplicate entries

#### Acceptance Criteria

1. WHEN the User clicks on an expense entry, THE Expense Tracker SHALL display an edit form with the current expense details
2. WHEN the User updates an expense entry, THE Expense Tracker SHALL save the changes to the SQLite Database
3. THE Expense Tracker SHALL provide a delete button for each expense entry
4. WHEN the User clicks the delete button, THE Expense Tracker SHALL display a confirmation dialog before deletion
5. WHEN the User confirms deletion, THE Expense Tracker SHALL remove the expense entry from the SQLite Database

### Requirement 7

**User Story:** As a User, I want the application to have a modern, stylish interface with gradients, so that using the app is visually pleasant and motivating

#### Acceptance Criteria

1. THE Expense Tracker SHALL use gradient backgrounds for the main interface elements
2. THE Expense Tracker SHALL use smooth transitions and animations for UI interactions
3. THE Expense Tracker SHALL maintain a consistent color scheme throughout the application
4. THE Expense Tracker SHALL use modern typography with appropriate font weights and sizes
5. THE Expense Tracker SHALL ensure all text has sufficient contrast against gradient backgrounds for readability

### Requirement 8

**User Story:** As a User, I want the application to store all data locally on my computer, so that my financial information remains private and accessible offline

#### Acceptance Criteria

1. THE Expense Tracker SHALL store all expense, subscription, and receipt data in a local SQLite Database file
2. THE Expense Tracker SHALL create the SQLite Database file in the user's application data directory on first launch
3. THE Expense Tracker SHALL function fully without an internet connection
4. WHEN the application starts, THE Expense Tracker SHALL load data from the local SQLite Database
5. THE Expense Tracker SHALL save all changes to the SQLite Database immediately upon user action
