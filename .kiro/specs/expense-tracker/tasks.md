# Implementation Plan

- [x] 1. Set up Rust dependencies and database infrastructure
  - Add rusqlite, chrono, and thiserror dependencies to Cargo.toml
  - Create database module structure (db/mod.rs, db/connection.rs, db/migrations.rs)
  - Implement database initialization logic with table creation and migrations
  - Implement app data directory path resolution for database file location
  - _Requirements: 8.2, 8.4_

- [x] 2. Implement data models and database operations
  - [x] 2.1 Create Rust model structs for Expense and Subscription
    - Define Expense, CreateExpenseDto, UpdateExpenseDto structs with serde serialization
    - Define Subscription, CreateSubscriptionDto, UpdateSubscriptionDto structs
    - _Requirements: 1.2, 3.2_
  
  - [x] 2.2 Implement expense database operations
    - Write SQL queries for creating, reading, updating, and deleting expenses
    - Implement expense CRUD functions in db module with proper error handling
    - Add date and category indexing for query optimization
    - _Requirements: 1.2, 1.3, 1.4, 6.2, 6.5_
  
  - [x] 2.3 Implement subscription database operations
    - Write SQL queries for subscription CRUD operations
    - Implement subscription management functions including status toggle
    - Add function to calculate total monthly subscription cost
    - _Requirements: 3.2, 3.3, 3.5_
  
  - [x] 2.4 Initialize categories table with predefined data
    - Create categories table schema with name, color, and icon fields
    - Insert initial category data (交通費, 飲食費, 通信費, 消耗品費, 接待交際費, その他)
    - _Requirements: 5.1_

- [x] 3. Implement Tauri command handlers
  - [x] 3.1 Create expense command handlers
    - Implement create_expense command with validation
    - Implement get_expenses command with month and category filtering
    - Implement update_expense and delete_expense commands
    - Add input validation for amount (positive number) and date (not future)
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 4.1, 4.3, 6.1, 6.2, 6.5_
  
  - [x] 3.2 Create subscription command handlers
    - Implement create_subscription and update_subscription commands
    - Implement get_subscriptions command with active_only filter
    - Implement toggle_subscription_status command
    - Implement get_monthly_subscription_total command
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  
  - [x] 3.3 Implement receipt file handling
    - Create save_receipt command to copy files to app data directory
    - Implement file validation (size limit 10MB, format PNG/JPG/PDF)
    - Generate unique filenames with expense_id and timestamp
    - Store receipt file path in database
    - _Requirements: 2.1, 2.2, 2.5_
  
  - [x] 3.4 Register all commands in main.rs
    - Add all expense and subscription commands to Tauri builder
    - Initialize AppState with database connection
    - _Requirements: 8.4, 8.5_

- [x] 4. Create TypeScript types and Tauri command wrappers
  - Define TypeScript interfaces for Expense, Subscription, Category, and DTOs
  - Create utility functions in lib/utils/tauri.ts to wrap Tauri invoke calls
  - Implement error handling wrapper for Tauri commands
  - _Requirements: All requirements (type safety)_

- [ ] 5. Implement core UI components with gradient styling
  - [ ] 5.1 Set up TailwindCSS configuration with gradient utilities
    - Configure custom gradient colors in app.css
    - Define category color variables
    - Set up responsive design utilities
    - _Requirements: 7.1, 7.3_
  
  - [ ] 5.2 Create ExpenseForm component
    - Build form with date picker, amount input, category dropdown, description textarea
    - Add receipt file upload with image preview
    - Implement form validation (positive amount, valid date, required category)
    - Add gradient styling to buttons and focus states
    - Emit onSave and onCancel events
    - _Requirements: 1.1, 1.3, 1.4, 2.1, 5.2, 5.3, 7.1, 7.2, 7.4_
  
  - [ ] 5.3 Create ExpenseItem component
    - Display expense details with category color coding
    - Show receipt thumbnail if available
    - Add edit and delete buttons with confirmation dialog
    - Apply gradient hover effects
    - _Requirements: 4.2, 5.5, 6.3, 6.4, 7.1, 7.2_
  
  - [ ] 5.4 Create ExpenseList component
    - Display expenses grouped by date in descending order
    - Show category totals and grand total for selected month
    - Integrate ExpenseItem components
    - Add smooth transitions for list updates
    - _Requirements: 4.1, 4.2, 4.4, 4.5, 7.2_
  
  - [ ] 5.5 Create SubscriptionForm component
    - Build form with name, amount, billing cycle, start date, category inputs
    - Implement validation for required fields
    - Add gradient styling consistent with ExpenseForm
    - _Requirements: 3.1, 5.2, 5.3, 7.1_
  
  - [ ] 5.6 Create SubscriptionList component
    - Display active and inactive subscriptions
    - Show monthly cost calculation for annual subscriptions
    - Add toggle for active/inactive status
    - Display total monthly subscription cost
    - Apply gradient styling and smooth animations
    - _Requirements: 3.3, 3.4, 3.5, 7.1, 7.2_
  
  - [ ] 5.7 Create CategoryFilter component
    - Build multi-select checkbox list with predefined categories
    - Apply category-specific color coding
    - Emit filter change events
    - _Requirements: 4.3, 5.1, 5.4, 5.5_
  
  - [ ] 5.8 Create MonthSelector component
    - Build month picker dropdown or calendar widget
    - Emit month change events
    - _Requirements: 4.3_
  
  - [ ] 5.9 Create ReceiptViewer modal component
    - Display full-size receipt images or PDFs in modal
    - Add zoom functionality for images
    - Include close button
    - _Requirements: 2.3, 2.4_

- [ ] 6. Build main application pages and layout
  - [ ] 6.1 Update +layout.svelte with gradient background
    - Apply global gradient background styling
    - Set up navigation structure
    - Configure Inter font family
    - _Requirements: 7.1, 7.3, 7.5_
  
  - [ ] 6.2 Create main dashboard page (+page.svelte)
    - Display monthly expense summary with category breakdown
    - Show subscription list with total monthly cost
    - Add quick action buttons for adding expenses and subscriptions
    - Apply gradient card styling
    - _Requirements: 3.5, 4.2, 4.5, 7.1_
  
  - [ ] 6.3 Create expenses page (expenses/+page.svelte)
    - Integrate MonthSelector and CategoryFilter components
    - Integrate ExpenseList component
    - Add floating action button for new expense with gradient styling
    - Show ExpenseForm in modal or side panel
    - _Requirements: 1.1, 4.1, 4.3, 5.4, 7.1, 7.2_

- [ ] 7. Implement state management with Svelte 5 runes
  - Create expenses.svelte.ts store using $state and $derived runes
  - Implement reactive state for expenses, subscriptions, selected month, and filters
  - Add functions to call Tauri commands and update state
  - Handle loading and error states
  - _Requirements: 1.5, 4.3, 8.4, 8.5_

- [ ] 8. Wire up frontend and backend integration
  - [ ] 8.1 Connect ExpenseForm to create_expense and update_expense commands
    - Call Tauri commands on form submission
    - Handle success and error responses with toast notifications
    - Update local state after successful operations
    - _Requirements: 1.2, 1.5, 6.2, 8.5_
  
  - [ ] 8.2 Connect ExpenseList to get_expenses and delete_expense commands
    - Load expenses on component mount and when filters change
    - Implement delete confirmation and command invocation
    - Update UI after deletion
    - _Requirements: 4.1, 4.3, 6.5, 8.4_
  
  - [ ] 8.3 Connect SubscriptionForm and SubscriptionList to subscription commands
    - Implement create, update, and toggle status operations
    - Load subscriptions and calculate monthly total
    - Update UI reactively
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 8.5_
  
  - [ ] 8.4 Implement receipt file upload integration
    - Use Tauri dialog API to select files
    - Call save_receipt command with selected file path
    - Display uploaded receipt thumbnail in ExpenseItem
    - Open ReceiptViewer modal on thumbnail click
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 9. Add validation and error handling throughout the application
  - Implement frontend validation for all form inputs
  - Display inline error messages for validation failures
  - Show toast notifications for Tauri command errors
  - Add error boundary for unexpected errors
  - Ensure all error messages are user-friendly in Japanese
  - _Requirements: 1.3, 1.4, 2.5, 5.3_

- [ ]* 10. Write integration tests for critical flows
  - [ ]* 10.1 Test expense creation, editing, and deletion flow
    - Verify expense data is correctly saved to database
    - Test validation rules for amount and date
    - _Requirements: 1.2, 1.3, 1.4, 6.2, 6.5_
  
  - [ ]* 10.2 Test subscription management flow
    - Verify subscription CRUD operations
    - Test monthly cost calculation
    - Test active/inactive toggle
    - _Requirements: 3.2, 3.3, 3.4, 3.5_
  
  - [ ]* 10.3 Test receipt file handling
    - Verify file upload and storage
    - Test file size and format validation
    - _Requirements: 2.1, 2.2, 2.5_
  
  - [ ]* 10.4 Test filtering and aggregation
    - Verify month and category filtering
    - Test total calculations
    - _Requirements: 4.1, 4.2, 4.3, 4.5_
