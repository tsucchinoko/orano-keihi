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
            fileExtension: 'app.tar.gz',
            description: 'macOS Intel'
        },
        {
            target: 'darwin',
            arch: 'aarch64',
            fileExtension: 'app.tar.gz',
            description: 'macOS Apple Silicon'
        },
        {
            target: 'windows',
            arch: 'x86_64',
            fileExtension: 'msi.zip',
            description: 'Windows 64bit'
        }
    ];
}

/**
 * ファイルの署名を生成（実際の署名ファイルが存在する場合）
 * GitHub Actionsでは、Tauriが自動的に署名ファイルを生成するため、
 * それを読み込んで使用します。
 * 
 * minisignの署名ファイル形式:
 * 1行目: untrusted comment: signature from minisign secret key
 * 2行目: base64エンコードされた署名データ
 * 
 * Tauriは2行目のbase64署名データのみを必要とします。
 */
function getSignature(filePath) {
    const signatureFilePath = `${filePath}.sig`;
    
    try {
        if (fs.existsSync(signatureFilePath)) {
            const signatureContent = fs.readFileSync(signatureFilePath, 'utf8').trim();
            
            // minisign署名ファイルは複数行で構成されている
            // 1行目: コメント行（"untrusted comment: ..."）
            // 2行目: base64エンコードされた署名データ
            const lines = signatureContent.split('\n');
            
            if (lines.length >= 2) {
                // 2行目以降を結合（改行を除去）
                const signature = lines.slice(1).join('').trim();
                console.log(`✅ 署名ファイルを読み込み: ${path.basename(signatureFilePath)}`);
                console.log(`   署名データ（最初の50文字）: ${signature.substring(0, 50)}...`);
                return signature;
            } else {
                console.warn(`⚠️  署名ファイルの形式が不正: ${signatureFilePath}`);
                console.warn(`   期待される形式: 1行目=コメント、2行目=署名データ`);
                console.warn(`   実際の行数: ${lines.length}`);
            }
        }
    } catch (error) {
        console.warn(`⚠️  署名ファイルの読み込みに失敗: ${signatureFilePath}`, error.message);
    }
    
    // 署名ファイルが見つからない場合はエラーをスロー
    console.error(`❌ 署名ファイルが見つかりません: ${signatureFilePath}`);
    console.error(`🔍 ファイルパス: ${filePath}`);
    console.error(`🔍 署名ファイルパス: ${signatureFilePath}`);
    
    // ディレクトリ内のファイルを表示
    const dir = path.dirname(filePath);
    if (fs.existsSync(dir)) {
        console.error(`📁 ディレクトリ内のファイル (${dir}):`);
        const files = fs.readdirSync(dir);
        files.forEach(f => console.error(`   - ${f}`));
    }
    
    throw new Error(
        `署名ファイルが見つかりません: ${signatureFilePath}\n` +
        `Tauriビルドプロセスで署名ファイルが正しく生成されているか確認してください。\n` +
        `GitHub Actionsの「Tauri署名ファイルの生成」ステップを確認してください。`
    );
}

/**
 * ダウンロードURLを生成
 */
function generateDownloadUrl(githubRepo, releaseTag, fileName) {
    return `https://github.com/${githubRepo}/releases/download/${releaseTag}/${fileName}`;
}

/**
 * アプリケーションファイル名を生成
 * GitHub Actionsで実際に生成されるファイル名に合わせる
 */
function generateFileName(target, arch, version, extension) {
    // 実際のTauriビルドで生成されるファイル名パターンに合わせる
    if (target === 'darwin') {
        // macOS: orano-keihi_1.0.0_x64.dmg または orano-keihi_1.0.0_aarch64.dmg
        const archSuffix = arch === 'x86_64' ? 'x64' : arch;
        return `orano-keihi_${version}_${archSuffix}.${extension}`;
    } else if (target === 'windows') {
        // Windows: orano-keihi_1.0.0_x64_ja-JP.msi
        return `orano-keihi_${version}_x64_ja-JP.${extension}`;
    }
    
    return `orano-keihi_${version}_${target}_${arch}.${extension}`;
}

/**
 * Tauri updater仕様に準拠したJSONマニフェストを生成
 */
function generateUpdateManifest(config, envInfo) {
    const fileName = generateFileName(config.target, config.arch, envInfo.version, config.fileExtension);
    
    // 実際のファイルパス（ビルド成果物の場所）
    const actualFilePath = getActualFilePath(config, fileName);
    
    console.log(`🔍 ${config.description}のファイルを確認中...`);
    console.log(`   期待されるファイル名: ${fileName}`);
    console.log(`   ファイルパス: ${actualFilePath}`);
    
    let finalFileName = fileName;
    let finalFilePath = actualFilePath;
    
    // ファイルの存在確認と実際のファイル名の取得
    if (!fs.existsSync(actualFilePath)) {
        // ファイルが見つからない場合、ディレクトリ内の類似ファイルを探す
        const dir = path.dirname(actualFilePath);
        if (fs.existsSync(dir)) {
            const files = fs.readdirSync(dir);
            console.log(`   ディレクトリ内のファイル: ${files.join(', ')}`);
            
            // 拡張子が一致するファイルを探す
            const matchingFiles = files.filter(f => f.endsWith(`.${config.fileExtension}`) && !f.endsWith(`.${config.fileExtension}.sig`));
            if (matchingFiles.length > 0) {
                finalFileName = matchingFiles[0];
                finalFilePath = path.join(dir, finalFileName);
                console.log(`   実際のファイル名を使用: ${finalFileName}`);
            } else {
                console.warn(`⚠️  ${config.fileExtension}ファイルが見つかりません: ${dir}`);
            }
        } else {
            console.warn(`⚠️  ディレクトリが見つかりません: ${dir}`);
        }
    }
    
    const downloadUrl = generateDownloadUrl(envInfo.githubRepo, envInfo.releaseTag, finalFileName);
    const signature = getSignature(finalFilePath);
    
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
 * GitHub Actionsの成果物構造に対応
 */
function getActualFilePath(config, fileName) {
    // GitHub Actionsでダウンロードされた成果物の構造に合わせる
    const artifactsBasePath = path.join(__dirname, '..', 'artifacts');
    
    if (config.target === 'darwin') {
        // MacOS成果物: artifacts/macos-artifacts/*.app.tar.gz
        const macosDir = path.join(artifactsBasePath, 'macos-artifacts');

        // 実際のファイル名を検索（バージョンやアーキテクチャが異なる場合に対応）
        if (fs.existsSync(macosDir)) {
            const files = fs.readdirSync(macosDir);
            const appFiles = files.filter(f => f.endsWith('.app.tar.gz') && !f.endsWith('.app.tar.gz.sig'));

            if (appFiles.length > 0) {
                // アーキテクチャに応じたファイルを選択
                let targetFile = appFiles[0]; // デフォルト

                if (config.arch === 'x86_64') {
                    // Intel Mac用ファイルを探す（x64またはx86_64を含む）
                    const intelFile = appFiles.find(f => f.includes('x64') || f.includes('x86_64'));
                    if (intelFile) targetFile = intelFile;
                } else if (config.arch === 'aarch64') {
                    // Apple Silicon用ファイルを探す（aarch64またはarm64を含む）
                    const armFile = appFiles.find(f => f.includes('aarch64') || f.includes('arm64'));
                    if (armFile) targetFile = armFile;
                }

                return path.join(macosDir, targetFile);
            }
        }

        return path.join(macosDir, fileName);
    } else if (config.target === 'windows') {
        // Windows成果物: artifacts/windows-artifacts/*.msi.zip
        const windowsDir = path.join(artifactsBasePath, 'windows-artifacts');

        // 実際のファイル名を検索
        if (fs.existsSync(windowsDir)) {
            const files = fs.readdirSync(windowsDir);
            const msiZipFiles = files.filter(f => f.endsWith('.msi.zip') && !f.endsWith('.msi.zip.sig'));

            if (msiZipFiles.length > 0) {
                return path.join(windowsDir, msiZipFiles[0]);
            }
        }

        return path.join(windowsDir, fileName);
    }
    
    return path.join(artifactsBasePath, fileName);
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
            
            try {
                // マニフェスト生成
                const manifest = generateUpdateManifest(config, envInfo);
                
                // 検証
                validateManifest(manifest, config);
                
                // ファイル保存
                const filePath = saveManifestFile(config, manifest);
                generatedFiles.push(filePath);
            } catch (error) {
                console.error(`❌ ${config.description}の処理中にエラーが発生: ${error.message}`);
                // 他のプラットフォームの処理を続行
                continue;
            }
        }
        
        console.log('\n' + '='.repeat(60));
        
        if (generatedFiles.length === 0) {
            console.error('❌ マニフェストファイルが生成されませんでした');
            console.error('🔍 トラブルシューティング:');
            console.error('   - ビルド成果物が正しい場所に配置されているか確認してください');
            console.error('   - artifacts/macos-artifacts/ と artifacts/windows-artifacts/ ディレクトリを確認してください');
            process.exit(1);
        }
        
        console.log('🎉 静的JSONファイル生成が完了しました！');
        console.log(`📊 生成されたファイル数: ${generatedFiles.length}/${platformConfigs.length}`);
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