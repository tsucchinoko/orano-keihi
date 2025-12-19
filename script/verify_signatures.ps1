# Windows コード署名検証スクリプト
# 使用方法: .\script\verify_signatures.ps1 -MsiFile "app.msi" [-DmgFile "app.dmg"]

param(
    [Parameter(Mandatory=$true)]
    [string]$MsiFile,
    
    [Parameter(Mandatory=$false)]
    [string]$DmgFile
)

Write-Host "🔍 Windows コード署名検証スクリプト" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green

# MSIファイルの検証
if (Test-Path $MsiFile) {
    Write-Host "📦 Windows msiファイルの署名を検証中: $MsiFile" -ForegroundColor Yellow
    
    # ファイル情報の表示
    $fileInfo = Get-Item $MsiFile
    Write-Host "ファイルサイズ: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor Cyan
    Write-Host "作成日時: $($fileInfo.CreationTime)" -ForegroundColor Cyan
    Write-Host "更新日時: $($fileInfo.LastWriteTime)" -ForegroundColor Cyan
    
    # SHA256ハッシュの計算
    $hash = Get-FileHash -Path $MsiFile -Algorithm SHA256
    Write-Host "SHA256ハッシュ: $($hash.Hash)" -ForegroundColor Cyan
    
    Write-Host ""
    
    # PowerShellでの署名検証
    Write-Host "🔐 PowerShellでの署名検証中..." -ForegroundColor Yellow
    try {
        $signature = Get-AuthenticodeSignature -FilePath $MsiFile
        
        Write-Host "署名ステータス: $($signature.Status)" -ForegroundColor $(
            switch ($signature.Status) {
                "Valid" { "Green" }
                "NotSigned" { "Red" }
                "HashMismatch" { "Red" }
                "NotTrusted" { "Yellow" }
                "UnknownError" { "Red" }
                default { "Yellow" }
            }
        )
        
        if ($signature.SignerCertificate) {
            Write-Host "署名者: $($signature.SignerCertificate.Subject)" -ForegroundColor Cyan
            Write-Host "発行者: $($signature.SignerCertificate.Issuer)" -ForegroundColor Cyan
            Write-Host "有効期限: $($signature.SignerCertificate.NotAfter)" -ForegroundColor Cyan
            Write-Host "拇印: $($signature.SignerCertificate.Thumbprint)" -ForegroundColor Cyan
            
            # 証明書チェーンの検証
            Write-Host ""
            Write-Host "🔗 証明書チェーンの検証中..." -ForegroundColor Yellow
            
            $chain = New-Object System.Security.Cryptography.X509Certificates.X509Chain
            $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
            $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::ExcludeRoot
            
            if ($chain.Build($signature.SignerCertificate)) {
                Write-Host "✅ 証明書チェーンが有効です" -ForegroundColor Green
                
                Write-Host "証明書チェーン:" -ForegroundColor Cyan
                foreach ($element in $chain.ChainElements) {
                    $cert = $element.Certificate
                    Write-Host "  - $($cert.Subject)" -ForegroundColor Gray
                }
            } else {
                Write-Host "❌ 証明書チェーンの検証に失敗しました" -ForegroundColor Red
                
                Write-Host "チェーンエラー:" -ForegroundColor Red
                foreach ($status in $chain.ChainStatus) {
                    Write-Host "  - $($status.Status): $($status.StatusInformation)" -ForegroundColor Red
                }
            }
        } else {
            Write-Host "❌ 署名証明書が見つかりません" -ForegroundColor Red
        }
        
        # タイムスタンプの確認
        if ($signature.TimeStamperCertificate) {
            Write-Host ""
            Write-Host "⏰ タイムスタンプ情報:" -ForegroundColor Yellow
            Write-Host "タイムスタンプ機関: $($signature.TimeStamperCertificate.Subject)" -ForegroundColor Cyan
        } else {
            Write-Host "⚠️ タイムスタンプが見つかりません" -ForegroundColor Yellow
        }
        
    } catch {
        Write-Host "❌ PowerShellでの署名検証に失敗しました: $($_.Exception.Message)" -ForegroundColor Red
    }
    
    Write-Host ""
    
    # signtoolでの検証（利用可能な場合）
    Write-Host "🛠️ signtoolでの署名検証中..." -ForegroundColor Yellow
    
    # Windows SDKのsigntoolを検索
    $signtoolPaths = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe",
        "${env:ProgramFiles}\Windows Kits\10\bin\*\x64\signtool.exe",
        "${env:ProgramFiles(x86)}\Microsoft SDKs\Windows\*\bin\signtool.exe",
        "${env:ProgramFiles}\Microsoft SDKs\Windows\*\bin\signtool.exe"
    )
    
    $signtool = $null
    foreach ($path in $signtoolPaths) {
        $found = Get-ChildItem -Path $path -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) {
            $signtool = $found.FullName
            break
        }
    }
    
    if ($signtool) {
        Write-Host "signtoolが見つかりました: $signtool" -ForegroundColor Cyan
        
        try {
            # 署名の検証
            $verifyResult = & $signtool verify /pa /v $MsiFile 2>&1
            
            if ($LASTEXITCODE -eq 0) {
                Write-Host "✅ signtoolでの署名検証に成功しました" -ForegroundColor Green
            } else {
                Write-Host "❌ signtoolでの署名検証に失敗しました" -ForegroundColor Red
            }
            
            Write-Host "signtool出力:" -ForegroundColor Gray
            $verifyResult | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
            
        } catch {
            Write-Host "❌ signtoolの実行に失敗しました: $($_.Exception.Message)" -ForegroundColor Red
        }
    } else {
        Write-Host "⚠️ signtoolが見つかりません（Windows SDKをインストールしてください）" -ForegroundColor Yellow
    }
    
} else {
    Write-Host "❌ msiファイルが見つかりません: $MsiFile" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "================================" -ForegroundColor Green

# DMGファイルの情報表示（Windowsでは制限あり）
if ($DmgFile -and (Test-Path $DmgFile)) {
    Write-Host "📦 MacOS dmgファイル情報: $DmgFile" -ForegroundColor Yellow
    
    $dmgInfo = Get-Item $DmgFile
    Write-Host "ファイルサイズ: $([math]::Round($dmgInfo.Length / 1MB, 2)) MB" -ForegroundColor Cyan
    
    $dmgHash = Get-FileHash -Path $DmgFile -Algorithm SHA256
    Write-Host "SHA256ハッシュ: $($dmgHash.Hash)" -ForegroundColor Cyan
    
    Write-Host ""
    Write-Host "⚠️ 注意: MacOS署名の詳細検証はmacOS環境で実行してください" -ForegroundColor Yellow
    Write-Host "macOS環境での検証コマンド:" -ForegroundColor Gray
    Write-Host "  codesign -dv --verbose=4 /path/to/app.app" -ForegroundColor Gray
    Write-Host "  spctl -a -vv /path/to/app.app" -ForegroundColor Gray
    
} elseif ($DmgFile) {
    Write-Host "⚠️ dmgファイルが見つかりません: $DmgFile" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🎯 署名検証が完了しました" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green

# 署名に関する推奨事項を表示
Write-Host ""
Write-Host "📋 署名に関する推奨事項:" -ForegroundColor Yellow
Write-Host "  ✓ EV（Extended Validation）証明書の使用を推奨" -ForegroundColor Gray
Write-Host "  ✓ タイムスタンプサーバーの使用を推奨" -ForegroundColor Gray
Write-Host "  ✓ 証明書の有効期限を定期的に確認" -ForegroundColor Gray
Write-Host "  ✓ 署名後はファイルを変更しない" -ForegroundColor Gray
Write-Host "  ✓ 配布前に署名の検証を実施" -ForegroundColor Gray