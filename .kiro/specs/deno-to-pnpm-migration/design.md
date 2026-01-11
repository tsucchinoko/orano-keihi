# 設計書

## 概要

このプロジェクトをDenoからpnpmに移行するための包括的な設計です。現在のDeno設定を完全に削除し、pnpmベースの開発環境に移行します。移行は段階的に行い、各ステップで動作確認を行います。

## アーキテクチャ

### 現在のアーキテクチャ
```
Deno Runtime
├── deno.json (タスク定義、依存関係)
├── package.json (Denoタスクへの委譲)
└── npm依存関係 (Denoが管理)
```

### 移行後のアーキテクチャ
```
Node.js + pnpm
├── package.json (直接的なnpmスクリプト)
├── pnpm-lock.yaml (ロックファイル)
├── node_modules/ (pnpmが管理)
└── .pnpmrc (pnpm設定)
```

## コンポーネントとインターフェース

### 1. パッケージ管理システム
- **現在**: Denoがnpm依存関係を管理
- **移行後**: pnpmが直接依存関係を管理
- **インターフェース**: package.json、pnpm-lock.yaml

### 2. タスクランナー
- **現在**: deno.jsonのtasksセクション
- **移行後**: package.jsonのscriptsセクション
- **インターフェース**: `pnpm run <task>`コマンド

### 3. 開発ツール統合
- **TypeScript**: tsconfig.jsonで設定（変更なし）
- **Biome**: package.jsonから直接実行
- **SvelteKit**: Viteを通じて実行
- **Tauri**: cargo tauriコマンド（変更なし）

### 4. CI/CD統合
- **GitHub Actions**: pnpmアクションを使用
- **キャッシュ**: pnpm-lock.yamlベース
- **ビルドスクリプト**: pnpmコマンドに更新

## データモデル

### package.json構造
```json
{
  "name": "orano-keihi",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "check:watch": "svelte-check --tsconfig ./tsconfig.json --watch",
    "tauri": "tauri",
    "fmt": "biome format --write .",
    "fmt:check": "biome format .",
    "lint": "biome lint --write .",
    "lint:check": "biome lint ."
  },
  "dependencies": { /* 既存の依存関係 */ },
  "devDependencies": { /* 既存の開発依存関係 */ }
}
```

### .pnpmrc設定
```
# パフォーマンス最適化
store-dir=~/.pnpm-store
verify-store-integrity=false

# Hoisting設定
hoist-pattern[]=*eslint*
hoist-pattern[]=*prettier*
hoist-pattern[]=*biome*

# Node.js互換性
node-linker=isolated
```

## 正確性プロパティ

*プロパティとは、システムのすべての有効な実行において真であるべき特性や動作のことです。これは人間が読める仕様と機械で検証可能な正確性保証の橋渡しとなります。*
プロパティ1: 依存関係インストールの一貫性
*任意の*有効なpackage.jsonに対して、pnpm installを実行すると、node_modulesディレクトリが作成され、pnpm-lock.yamlファイルが生成されること
**検証対象: 要件 1.1**

プロパティ2: 依存関係追加の整合性
*任意の*有効なnpmパッケージ名に対して、pnpm addを実行すると、package.jsonとpnpm-lock.yamlの両方が更新されること
**検証対象: 要件 1.2, 6.5**

## エラーハンドリング

### 1. 移行プロセスのエラー
- **バックアップ作成**: 移行前に重要ファイルのバックアップを作成
- **段階的ロールバック**: 各ステップで問題が発生した場合の復旧手順
- **依存関係の競合**: pnpmとDenoの依存関係が競合する場合の解決方法

### 2. 実行時エラー
- **コマンド実行失敗**: pnpmコマンドが失敗した場合の詳細なエラーメッセージ
- **ビルドエラー**: 移行後のビルドが失敗した場合の診断手順
- **CI/CDエラー**: GitHub Actionsでの実行エラーの対処法

### 3. 設定エラー
- **package.json構文エラー**: JSON構文エラーの検出と修正
- **スクリプトエラー**: npmスクリプトの実行エラーの対処
- **パス解決エラー**: モジュール解決の問題の診断

## テスト戦略

### 単体テスト
- 設定ファイルの構文検証
- スクリプトコマンドの存在確認
- ファイル構造の検証

### 統合テスト
- 完全な開発ワークフローのテスト
- CI/CDパイプラインの動作確認
- Tauriビルドプロセスの検証

### プロパティベーステスト
- **ライブラリ**: Node.jsの標準テストフレームワーク（Jest/Vitest）を使用
- **最小実行回数**: 各プロパティテストは最低100回実行
- **タグ付け**: 各プロパティベーステストは対応する設計書のプロパティを明示的に参照
- **フォーマット**: '**Feature: deno-to-pnpm-migration, Property {number}: {property_text}**'

プロパティベーステストは以下の要件を満たす必要があります：
- 各正確性プロパティは単一のプロパティベーステストで実装される
- テストは実装に近い位置に配置し、エラーを早期に発見できるようにする
- 各テストは設計書のプロパティ番号で注釈される
- 要件文書の条項番号も注釈に含める

単体テストとプロパティテストは相補的です：
- 単体テストは具体的な例、エッジケース、エラー条件を検証
- プロパティテストはすべての入力にわたって普遍的なプロパティを検証
- 両方のタイプのテストが包括的なカバレッジを提供：単体テストは具体的なバグを捕捉し、プロパティテストは一般的な正確性を検証