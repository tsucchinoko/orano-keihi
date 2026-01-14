# ローカルビルド署名トラブルシューティング

## 発生した問題

ローカル環境でのmacOSアプリケーションのコード署名時に`errSecInternalComponent`エラーが発生します。

```
Warning: unable to build chain to self-signed root for signer "Developer ID Application: ..."
errSecInternalComponent
```

## 問題の原因

1. **証明書チェーンの不完全性**: .p12ファイルにApple中間証明書が含まれていない
2. **キーチェーンアクセス**: Tauriのビルドプロセスが適切に証明書にアクセスできない
3. **証明書の形式**: エクスポートされた.p12ファイルに秘密鍵と証明書の関連付けに問題がある可能性

## 試行した解決策

### 1. Apple中間証明書のダウンロードとインポート
```bash
curl -s https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer -o DeveloperIDG2CA.cer
security import DeveloperIDG2CA.cer -k build.keychain -T /usr/bin/codesign -A
```
**結果**: エラーが継続

### 2. loginキーチェーンの検索パスへの追加
```bash
security list-keychains -d user -s build.keychain login.keychain
```
**結果**: 証明書は検出されるが、署名時にエラー

### 3. .p12ファイルを使用しない方法
loginキーチェーンの証明書を直接使用する試み
**結果**: 同じエラーが発生

## 推奨される解決策

### オプション1: GitHub Actionsを使用（推奨）

`.github/workflows/release.yml`は既に正しく設定されており、以下の手順で署名されたビルドを作成できます：

1. コードを`release`ブランチにプッシュ
2. GitHub Actionsが自動的にビルドと署名を実行
3. 署名されたDMGファイルがGitHubリリースに添付される

**利点**:
- 証明書管理が簡単（GitHub Secretsに保存）
- クリーンな環境で毎回ビルド
- 中間証明書が正しくインポートされる

### オプション2: 証明書を再エクスポート

キーチェーンアクセスから証明書を再エクスポートする際、以下の点に注意：

1. **証明書と秘密鍵の両方を選択**してエクスポート
2. **証明書チェーン全体を含める**オプションを確認
3. 強力なパスワードを設定

手順：
```
1. キーチェーンアクセスを開く
2. 「自分の証明書」カテゴリを選択
3. "Developer ID Application" 証明書を見つける
4. 証明書を展開して、配下の秘密鍵も表示させる
5. 証明書と秘密鍵の両方を選択（Cmd+クリック）
6. 右クリック → "2項目を書き出す..."
7. .p12形式で保存
8. パスワードを設定
9. base64エンコード: base64 -i certificate.p12 -o certificate.p12.base64
```

### オプション3: アドホック署名で開発

開発環境では、アドホック署名（現在の状態）でも動作します：

```bash
# ダウンロードしたアプリの検疫属性を削除
xattr -d com.apple.quarantine /path/to/downloaded.dmg
```

**注意**: 配布用には適していません。

## 現在のスクリプトの状態

`script/build-and-sign-local.sh`は以下のように修正されています：

1. ✅ loginキーチェーンを検索パスに追加
2. ✅ Apple中間証明書（G2）のダウンロードとインポート
3. ✅ 証明書の自動検出
4. ✅ クリーンアップ処理（trap設定）

## 次のステップ

1. **短期的**: GitHub Actionsを使用してリリースビルドを作成
2. **長期的**: 証明書を正しくエクスポートし直してローカルビルドを修正

## 参考情報

- macOS証明書要件: https://developer.apple.com/support/certificates/
- Tauriコード署名: https://tauri.app/v1/guides/distribution/sign-macos/
- GitHub Actions workflow: `.github/workflows/release.yml`
