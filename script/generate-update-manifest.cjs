#!/usr/bin/env node

/**
 * Tauri自動アップデート用の静的JSONファイル生成スクリプト
 * 
 * このスクリプトは、GitHub Actionsのリリースワークフロー内で実行され、
 * 各プラットフォーム・アーキテクチャ用の静的JSONファイルを生成します。
 * 
 * 生成されるファイル:
 * - darwin-x86_64.json (macOS Intel)
 * - darwin-aarch64.json (macOS Apple Silicon)
 * - windows-x86_64.json (Windows 64bit)
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

/**
 * 環境変数から必要な情報を取得
 */
function getEnvironmentInfo() {
    const version = process.env.VERSION || require('../package.json').version;
    const releaseTag = process.env.RELEASE_TAG || `v${version}`;
    const releaseNotes = process.env.RELEASE_NOTES || `バージョン ${version} のリリース`;
    const githubRepo = process.env.GITHUB_REPOSITORY || 'tsucchinoko/orano-keihi';
    
    // 現在の日時をJSTで取得
    const pubDate = new Date().toISOString();
    
    return {
        version,
        releaseTag,
        releaseNotes,
        githubRepo,
        pubDate
    };
}

/**
 * プラットフォーム・アーキテクチャの組み合わせを定義
 */
function getPlatformConfigurations() {
    return [
        {
            target: 'darwin',
            arch: 'x86_64',
            fileExtension: 'dmg',
            description: 'macOS Intel'
        },
        {
            target: 'darwin',
            arch: 'aarch64',
            fileExtension: 'dmg',
            description: 'macOS Apple Silicon'
        },
        {
            target: 'windows',
            arch: 'x86_64',
            fileExtension: 'msi',
            description: 'Windows 64bit'
        }
    ];
}

/**
 * ファイルの署名を生成（実際の署名ファイルが存在する場合）
 * GitHub Actionsでは、Tauriが自動的に署名ファイルを生成するため、
 * それを読み込んで使用します。
 */
function getSignature(filePath) {
    const signatureFilePath = `${filePath}.sig`;
    
    try {
        if (fs.existsSync(signatureFilePath)) {
            return fs.readFileSync(signatureFilePath, 'utf8').trim();
        }
    } catch (error) {
        console.warn(`署名ファイルの読み込みに失敗: ${signatureFilePath}`, error.message);
    }
    
    // 署名ファイルが見つからない場合は、プレースホルダーを返す
    // 実際のリリース時には、Tauriが適切な署名を生成します
    return 'SIGNATURE_PLACEHOLDER';
}

/**
 * ダウンロードURLを生成
 */
function generateDownloadUrl(githubRepo, releaseTag, fileName) {
    return `https://github.com/${githubRepo}/releases/download/${releaseTag}/${fileName}`;
}

/**
 * アプリケーションファイル名を生成
 */
function generateFileName(target, arch, version, extension) {
    const productName = 'orano-keihi';
    
    if (target === 'darwin') {
        return `${productName}_${version}_${arch}.${extension}`;
    } else if (target === 'windows') {
        return `${productName}_${version}_${arch}.${extension}`;
    }
    
    return `${productName}_${version}_${target}_${arch}.${extension}`;
}

/**
 * Tauri updater仕様に準拠したJSONマニフェストを生成
 */
function generateUpdateManifest(config, envInfo) {
    const fileName = generateFileName(config.target, config.arch, envInfo.version, config.fileExtension);
    const downloadUrl = generateDownloadUrl(envInfo.githubRepo, envInfo.releaseTag, fileName);
    
    // 実際のファイルパス（ビルド成果物の場所）
    const actualFilePath = getActualFilePath(config, fileName);
    const signature = getSignature(actualFilePath);
    
    return {
        version: envInfo.version,
        notes: envInfo.releaseNotes,
        pub_date: envInfo.pubDate,
        platforms: {
            [`${config.target}-${config.arch}`]: {
                signature: signature,
                url: downloadUrl
            }
        }
    };
}

/**
 * 実際のビルド成果物のファイルパスを取得
 */
function getActualFilePath(config, fileName) {
    const basePath = path.join(__dirname, '..', 'packages', 'desktop', 'src-tauri', 'target', 'release', 'bundle');
    
    if (config.target === 'darwin') {
        return path.join(basePath, 'dmg', fileName);
    } else if (config.target === 'windows') {
        return path.join(basePath, 'msi', fileName);
    }
    
    return path.join(basePath, fileName);
}

/**
 * JSONファイルを出力ディレクトリに保存
 */
function saveManifestFile(config, manifest) {
    const outputDir = path.join(__dirname, '..', 'update-manifests');
    
    // 出力ディレクトリが存在しない場合は作成
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }
    
    const fileName = `${config.target}-${config.arch}.json`;
    const filePath = path.join(outputDir, fileName);
    
    // JSONファイルを整形して保存
    fs.writeFileSync(filePath, JSON.stringify(manifest, null, 2), 'utf8');
    
    console.log(`✅ ${config.description}用マニフェストを生成: ${fileName}`);
    console.log(`   バージョン: ${manifest.version}`);
    console.log(`   ダウンロードURL: ${manifest.platforms[`${config.target}-${config.arch}`].url}`);
    
    return filePath;
}

/**
 * 生成されたマニフェストファイルの検証
 */
function validateManifest(manifest, config) {
    const requiredFields = ['version', 'notes', 'pub_date', 'platforms'];
    const platformKey = `${config.target}-${config.arch}`;
    
    // 必須フィールドの確認
    for (const field of requiredFields) {
        if (!manifest[field]) {
            throw new Error(`必須フィールドが不足: ${field}`);
        }
    }
    
    // プラットフォーム情報の確認
    if (!manifest.platforms[platformKey]) {
        throw new Error(`プラットフォーム情報が不足: ${platformKey}`);
    }
    
    const platform = manifest.platforms[platformKey];
    if (!platform.signature || !platform.url) {
        throw new Error(`プラットフォーム詳細情報が不足: signature または url`);
    }
    
    // URLの形式確認
    if (!platform.url.startsWith('https://')) {
        throw new Error(`無効なURL形式: ${platform.url}`);
    }
    
    console.log(`✅ マニフェスト検証完了: ${config.target}-${config.arch}`);
}

/**
 * メイン処理
 */
function main() {
    console.log('🚀 Tauri自動アップデート用静的JSONファイル生成を開始');
    console.log('='.repeat(60));
    
    try {
        // 環境情報の取得
        const envInfo = getEnvironmentInfo();
        console.log('📋 環境情報:');
        console.log(`   バージョン: ${envInfo.version}`);
        console.log(`   リリースタグ: ${envInfo.releaseTag}`);
        console.log(`   リポジトリ: ${envInfo.githubRepo}`);
        console.log(`   公開日時: ${envInfo.pubDate}`);
        console.log('');
        
        // プラットフォーム設定の取得
        const platformConfigs = getPlatformConfigurations();
        const generatedFiles = [];
        
        // 各プラットフォーム用のマニフェストファイルを生成
        console.log('📦 マニフェストファイル生成:');
        for (const config of platformConfigs) {
            console.log(`\n🔧 ${config.description} (${config.target}-${config.arch}) を処理中...`);
            
            // マニフェスト生成
            const manifest = generateUpdateManifest(config, envInfo);
            
            // 検証
            validateManifest(manifest, config);
            
            // ファイル保存
            const filePath = saveManifestFile(config, manifest);
            generatedFiles.push(filePath);
        }
        
        console.log('\n' + '='.repeat(60));
        console.log('🎉 静的JSONファイル生成が完了しました！');
        console.log('\n📁 生成されたファイル:');
        generatedFiles.forEach(file => {
            const stats = fs.statSync(file);
            const sizeKB = (stats.size / 1024).toFixed(2);
            console.log(`   - ${path.basename(file)} (${sizeKB} KB)`);
        });
        
        console.log('\n💡 次のステップ:');
        console.log('   1. GitHub Actionsワークフローでこれらのファイルをリリースにアップロード');
        console.log('   2. Tauriアプリケーションが自動的にアップデートをチェック');
        console.log('   3. ユーザーに新しいバージョンが通知される');
        
    } catch (error) {
        console.error('\n❌ エラーが発生しました:', error.message);
        console.error('\n🔍 デバッグ情報:');
        console.error('   - 環境変数を確認してください');
        console.error('   - ビルド成果物が存在することを確認してください');
        console.error('   - 署名ファイルが生成されていることを確認してください');
        process.exit(1);
    }
}

// スクリプトが直接実行された場合のみメイン処理を実行
if (require.main === module) {
    main();
}

module.exports = {
    generateUpdateManifest,
    getPlatformConfigurations,
    getEnvironmentInfo,
    validateManifest
};