#!/bin/bash

# コード署名検証スクリプト
# 使用方法: ./script/verify_signatures.sh [dmg_file] [msi_file]

set -e

echo "🔍 コード署名検証スクリプト"
echo "================================"

# 引数の確認
if [ $# -lt 1 ]; then
    echo "使用方法: $0 <dmg_file> [msi_file]"
    echo "例: $0 app.dmg app.msi"
    exit 1
fi

DMG_FILE="$1"
MSI_FILE="$2"

# MacOS dmgファイルの検証
if [ -n "$DMG_FILE" ] && [ -f "$DMG_FILE" ]; then
    echo "📦 MacOS dmgファイルの署名を検証中: $DMG_FILE"
    
    # dmgファイルをマウント
    echo "dmgファイルをマウント中..."
    MOUNT_POINT=$(hdiutil attach "$DMG_FILE" -nobrowse -quiet | grep "/Volumes" | awk '{print $3}')
    
    if [ -z "$MOUNT_POINT" ]; then
        echo "❌ dmgファイルのマウントに失敗しました"
        exit 1
    fi
    
    echo "マウントポイント: $MOUNT_POINT"
    
    # アプリケーションファイルを検索
    APP_FILE=$(find "$MOUNT_POINT" -name "*.app" -type d | head -1)
    
    if [ -z "$APP_FILE" ]; then
        echo "❌ アプリケーションファイルが見つかりません"
        hdiutil detach "$MOUNT_POINT" -quiet
        exit 1
    fi
    
    echo "アプリケーションファイル: $APP_FILE"
    
    # 署名の検証
    echo "🔐 コード署名を検証中..."
    if codesign -dv --verbose=4 "$APP_FILE" 2>&1; then
        echo "✅ コード署名が有効です"
        
        # 署名の詳細情報を表示
        echo ""
        echo "📋 署名詳細情報:"
        codesign -dv "$APP_FILE" 2>&1 | grep -E "(Identifier|Authority|TeamIdentifier|Timestamp)"
        
        # Gatekeeperの検証
        echo ""
        echo "🛡️ Gatekeeperの検証中..."
        if spctl -a -vv "$APP_FILE" 2>&1; then
            echo "✅ Gatekeeperの検証に成功しました"
        else
            echo "⚠️ Gatekeeperの検証に失敗しました（公証が必要な可能性があります）"
        fi
        
    else
        echo "❌ コード署名の検証に失敗しました"
    fi
    
    # dmgファイル自体の署名も確認
    echo ""
    echo "🔐 dmgファイル自体の署名を確認中..."
    if codesign -dv --verbose=4 "$DMG_FILE" 2>&1; then
        echo "✅ dmgファイルの署名が有効です"
    else
        echo "⚠️ dmgファイルは署名されていません"
    fi
    
    # dmgファイルをアンマウント
    hdiutil detach "$MOUNT_POINT" -quiet
    echo "dmgファイルをアンマウントしました"
    
else
    echo "⚠️ MacOS dmgファイルが指定されていないか、ファイルが存在しません: $DMG_FILE"
fi

echo ""
echo "================================"

# Windows msiファイルの検証（macOS上では制限あり）
if [ -n "$MSI_FILE" ] && [ -f "$MSI_FILE" ]; then
    echo "📦 Windows msiファイルの署名情報: $MSI_FILE"
    
    # ファイルサイズとハッシュを表示
    echo "ファイルサイズ: $(du -h "$MSI_FILE" | cut -f1)"
    echo "SHA256ハッシュ: $(shasum -a 256 "$MSI_FILE" | cut -d' ' -f1)"
    
    # msiファイルの基本情報を表示
    if command -v file >/dev/null 2>&1; then
        echo "ファイル形式: $(file "$MSI_FILE")"
    fi
    
    echo ""
    echo "⚠️ 注意: Windows署名の詳細検証はWindows環境で実行してください"
    echo "Windows環境での検証コマンド:"
    echo "  Get-AuthenticodeSignature \"$MSI_FILE\""
    echo "  signtool verify /pa /v \"$MSI_FILE\""
    
else
    echo "⚠️ Windows msiファイルが指定されていないか、ファイルが存在しません: $MSI_FILE"
fi

echo ""
echo "🎯 署名検証が完了しました"
echo "================================"