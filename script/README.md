# Tauri自動アップデート用スクリプト

このディレクトリには、Tauri自動アップデート機能で使用する静的JSONファイル生成スクリプトが含まれています。

## generate-update-manifest.cjs

### 概要

GitHub Actionsのリリースワークフロー内で実行され、各プラットフォーム・アーキテクチャ用の静的JSONファイルを生成するスクリプトです。

### 生成されるファイル

- `darwin-x86_64.json` - macOS Intel用
- `darwin-aarch64.json` - macOS Apple Silicon用  
- `windows-x86_64.json` - Windows 64bit用

### 使用方法

#### 基本実行

```bash
node script/generate-update-manifest.cjs
```

#### 環境変数を指定した実行

```bash
VERSION=1.2.3 \
RELEASE_TAG=v1.2.3-20250107-120000 \
RELEASE_NOTES="新機能追加とバグ修正" \
GITHUB_REPOSITORY=tsucchinoko/orano-keihi \
node script/generate-update-manifest.cjs
```

### 環境変数

| 変数名 | 説明 | デフォルト値 |
|--------|------|-------------|
| `VERSION` | アプリケーションのバージョン | package.jsonから取得 |
| `RELEASE_TAG` | GitHubリリースのタグ名 | `v${VERSION}` |
| `RELEASE_NOTES` | リリースノート | `バージョン ${VERSION} のリリース` |
| `GITHUB_REPOSITORY` | GitHubリポジトリ名 | `tsucchinoko/orano-keihi` |

### 出力

スクリプトは `update-manifests/` ディレクトリに以下のファイルを生成します：

```
update-manifests/
├── darwin-x86_64.json
├── darwin-aarch64.json
└── windows-x86_64.json
```

### JSONファイル形式

生成されるJSONファイルは、Tauri updater仕様に準拠した以下の形式です：

```json
{
  "version": "1.2.3",
  "notes": "新機能追加とバグ修正",
  "pub_date": "2026-01-07T08:10:17.681Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "SIGNATURE_PLACEHOLDER",
      "url": "https://github.com/tsucchinoko/orano-keihi/releases/download/v1.2.3-20250107-120000/orano-keihi_1.2.3_x86_64.dmg"
    }
  }
}
```

### GitHub Actionsでの使用

GitHub Actionsワークフローでは、以下のようにスクリプトを実行します：

```yaml
- name: 静的JSONファイルの生成
  run: node script/generate-update-manifest.cjs
  env:
    VERSION: ${{ needs.version-tag.outputs.VERSION }}
    RELEASE_TAG: ${{ needs.version-tag.outputs.RELEASE_TAG }}
    RELEASE_NOTES: "リリースノート内容"
    GITHUB_REPOSITORY: ${{ github.repository }}

- name: JSONファイルのリリースへのアップロード
  uses: actions/upload-release-asset@v1
  with:
    upload_url: ${{ steps.create_release.outputs.upload_url }}
    asset_path: ./update-manifests/
    asset_name: update-manifests
    asset_content_type: application/json
```

### 署名について

実際のリリース時には、Tauriが自動的に署名ファイル（`.sig`）を生成します。スクリプトは、対応する署名ファイルが存在する場合はそれを読み込み、存在しない場合はプレースホルダーを使用します。

### トラブルシューティング

#### 署名ファイルが見つからない

```
署名ファイルの読み込みに失敗: /path/to/file.sig
```

これは正常な動作です。実際のビルド時にTauriが署名ファイルを生成するため、開発時にはプレースホルダーが使用されます。

#### 必須フィールドが不足

```
必須フィールドが不足: version
```

環境変数またはpackage.jsonの設定を確認してください。

#### 無効なURL形式

```
無効なURL形式: http://example.com/file.dmg
```

URLはHTTPS形式である必要があります。GITHUB_REPOSITORYの設定を確認してください。