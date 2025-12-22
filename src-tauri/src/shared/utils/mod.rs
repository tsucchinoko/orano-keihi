use crate::shared::errors::{AppError, AppResult};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;

/// 日付文字列のバリデーション
///
/// # 引数
/// * `date_str` - 日付文字列（YYYY-MM-DD形式）
///
/// # 戻り値
/// 有効な日付の場合はOk(())、無効な場合はエラー
///
/// # バリデーション規則
/// - YYYY-MM-DD形式であること
/// - 実在する日付であること
/// - 1900年以降、2100年以前であること
pub fn validate_date(date_str: &str) -> AppResult<()> {
    // 基本的な形式チェック
    if date_str.len() != 10 {
        return Err(AppError::validation(
            "日付はYYYY-MM-DD形式で入力してください",
        ));
    }

    // ハイフンの位置チェック
    if (date_str.chars().nth(4) != Some('-')) || (date_str.chars().nth(7) != Some('-')) {
        return Err(AppError::validation(
            "日付はYYYY-MM-DD形式で入力してください",
        ));
    }

    // 日付として解析可能かチェック
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::validation("無効な日付です"))?;

    // 年の範囲チェック
    let year = date.year();
    if !(1900..=2100).contains(&year) {
        return Err(AppError::validation(
            "日付は1900年から2100年の間で入力してください",
        ));
    }

    Ok(())
}

/// 金額のバリデーション
///
/// # 引数
/// * `amount` - 金額
///
/// # 戻り値
/// 有効な金額の場合はOk(())、無効な場合はエラー
///
/// # バリデーション規則
/// - 正の数値であること
/// - 10桁以内であること（9,999,999,999円まで）
/// - 小数点以下は2桁まで
pub fn validate_amount(amount: f64) -> AppResult<()> {
    // 正の数値チェック
    if amount <= 0.0 {
        return Err(AppError::validation("金額は正の数値で入力してください"));
    }

    // 上限チェック（10桁以内）
    if amount >= 10_000_000_000.0 {
        return Err(AppError::validation("金額は10桁以内で入力してください"));
    }

    // 無限大・NaNチェック
    if !amount.is_finite() {
        return Err(AppError::validation("無効な金額です"));
    }

    // 小数点以下の桁数チェック（2桁まで）
    let amount_str = format!("{amount:.10}"); // 十分な精度で文字列化
    if let Some(decimal_pos) = amount_str.find('.') {
        let decimal_part = &amount_str[decimal_pos + 1..];
        let significant_decimals = decimal_part.trim_end_matches('0');
        if significant_decimals.len() > 2 {
            return Err(AppError::validation(
                "金額は小数点以下2桁まで入力してください",
            ));
        }
    }

    Ok(())
}

/// 文字列の長さバリデーション
///
/// # 引数
/// * `text` - 検証対象の文字列
/// * `max_length` - 最大文字数
/// * `field_name` - フィールド名（エラーメッセージ用）
///
/// # 戻り値
/// 有効な長さの場合はOk(())、無効な場合はエラー
pub fn validate_text_length(text: &str, max_length: usize, field_name: &str) -> AppResult<()> {
    let char_count = text.chars().count();
    if char_count > max_length {
        return Err(AppError::validation(format!(
            "{field_name}は{max_length}文字以内で入力してください（現在: {char_count}文字）"
        )));
    }
    Ok(())
}

/// 必須フィールドのバリデーション
///
/// # 引数
/// * `text` - 検証対象の文字列
/// * `field_name` - フィールド名（エラーメッセージ用）
///
/// # 戻り値
/// 空でない場合はOk(())、空の場合はエラー
pub fn validate_required_field(text: &str, field_name: &str) -> AppResult<()> {
    if text.trim().is_empty() {
        return Err(AppError::validation(format!("{field_name}は必須項目です")));
    }
    Ok(())
}

/// カテゴリ名のバリデーション
///
/// # 引数
/// * `category` - カテゴリ名
///
/// # 戻り値
/// 有効なカテゴリ名の場合はOk(())、無効な場合はエラー
///
/// # バリデーション規則
/// - 必須項目であること
/// - 50文字以内であること
/// - 空白のみでないこと
pub fn validate_category(category: &str) -> AppResult<()> {
    validate_required_field(category, "カテゴリ")?;
    validate_text_length(category, 50, "カテゴリ")?;
    Ok(())
}

/// 説明文のバリデーション
///
/// # 引数
/// * `description` - 説明文（Option）
///
/// # 戻り値
/// 有効な説明文の場合はOk(())、無効な場合はエラー
///
/// # バリデーション規則
/// - 500文字以内であること（Noneの場合は有効）
pub fn validate_description(description: &Option<String>) -> AppResult<()> {
    if let Some(desc) = description {
        validate_text_length(desc, 500, "説明")?;
    }
    Ok(())
}

/// 現在の日時をJST（日本標準時）で取得
///
/// # 戻り値
/// JST形式のRFC3339文字列
pub fn get_current_jst_timestamp() -> String {
    let now_jst = Utc::now().with_timezone(&Tokyo);
    now_jst.to_rfc3339()
}

/// 日付文字列をJSTのDateTimeに変換
///
/// # 引数
/// * `date_str` - 日付文字列（YYYY-MM-DD形式）
///
/// # 戻り値
/// JST形式のDateTime、または変換失敗時はエラー
pub fn parse_date_to_jst_datetime(date_str: &str) -> AppResult<DateTime<chrono_tz::Tz>> {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::validation("日付の形式が正しくありません"))?;

    // 日付の開始時刻（00:00:00）をJSTで作成
    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::validation("日付の変換に失敗しました"))?;

    // JSTタイムゾーンを適用
    let jst_datetime = Tokyo
        .from_local_datetime(&naive_datetime)
        .single()
        .ok_or_else(|| AppError::validation("JSTタイムゾーンの適用に失敗しました"))?;

    Ok(jst_datetime)
}

/// 今日の日付をYYYY-MM-DD形式で取得（JST基準）
///
/// # 戻り値
/// 今日の日付文字列
pub fn get_today_date_jst() -> String {
    let now_jst = Utc::now().with_timezone(&Tokyo);
    now_jst.format("%Y-%m-%d").to_string()
}

/// URLのバリデーション（HTTPS必須）
///
/// # 引数
/// * `url` - URL文字列
///
/// # 戻り値
/// 有効なHTTPS URLの場合はOk(())、無効な場合はエラー
pub fn validate_https_url(url: &str) -> AppResult<()> {
    if !url.starts_with("https://") {
        return Err(AppError::validation("URLはHTTPS形式である必要があります"));
    }

    // 基本的なURL形式チェック
    if url.len() < 12 {
        // "https://a.b" の最小長
        return Err(AppError::validation("無効なURL形式です"));
    }

    // ドメイン部分の存在チェック
    let domain_part = &url[8..]; // "https://" を除く
    if !domain_part.contains('.') {
        return Err(AppError::validation("無効なURL形式です"));
    }

    Ok(())
}

/// 文字列の正規化（前後の空白を削除）
///
/// # 引数
/// * `text` - 正規化対象の文字列
///
/// # 戻り値
/// 正規化された文字列
pub fn normalize_string(text: &str) -> String {
    text.trim().to_string()
}

/// 金額を文字列形式でフォーマット（カンマ区切り）
///
/// # 引数
/// * `amount` - 金額
///
/// # 戻り値
/// フォーマットされた金額文字列
pub fn format_amount(amount: f64) -> String {
    // 小数点以下が0の場合は整数として表示
    if amount.fract() == 0.0 {
        format!("{amount:.0}")
    } else {
        format!("{amount:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_date() {
        // 有効な日付
        assert!(validate_date("2024-01-01").is_ok());
        assert!(validate_date("2024-12-31").is_ok());
        assert!(validate_date("2000-02-29").is_ok()); // うるう年

        // 無効な日付
        assert!(validate_date("2024-13-01").is_err()); // 無効な月
        assert!(validate_date("2024-02-30").is_err()); // 無効な日
        assert!(validate_date("2023-02-29").is_err()); // 非うるう年
        assert!(validate_date("24-01-01").is_err()); // 形式エラー
        assert!(validate_date("2024/01/01").is_err()); // 区切り文字エラー
        assert!(validate_date("1899-01-01").is_err()); // 年の範囲外
        assert!(validate_date("2101-01-01").is_err()); // 年の範囲外
    }

    #[test]
    fn test_validate_amount() {
        // 有効な金額
        assert!(validate_amount(1.0).is_ok());
        assert!(validate_amount(100.50).is_ok());
        assert!(validate_amount(9999999999.0).is_ok());
        assert!(validate_amount(0.01).is_ok());

        // 無効な金額
        assert!(validate_amount(0.0).is_err()); // ゼロ
        assert!(validate_amount(-1.0).is_err()); // 負の数
        assert!(validate_amount(10000000000.0).is_err()); // 上限超過
        assert!(validate_amount(f64::INFINITY).is_err()); // 無限大
        assert!(validate_amount(f64::NAN).is_err()); // NaN
        assert!(validate_amount(1.234).is_err()); // 小数点以下3桁
    }

    #[test]
    fn test_validate_text_length() {
        // 有効な長さ
        assert!(validate_text_length("短いテキスト", 10, "テスト").is_ok());
        assert!(validate_text_length("", 10, "テスト").is_ok());

        // 無効な長さ
        assert!(validate_text_length("これは非常に長いテキストです", 5, "テスト").is_err());
    }

    #[test]
    fn test_validate_required_field() {
        // 有効な値
        assert!(validate_required_field("有効な値", "テスト").is_ok());
        assert!(validate_required_field("  有効な値  ", "テスト").is_ok()); // 前後の空白は許可

        // 無効な値
        assert!(validate_required_field("", "テスト").is_err());
        assert!(validate_required_field("   ", "テスト").is_err()); // 空白のみ
    }

    #[test]
    fn test_validate_category() {
        // 有効なカテゴリ
        assert!(validate_category("交通費").is_ok());
        assert!(validate_category("飲食費").is_ok());

        // 無効なカテゴリ
        assert!(validate_category("").is_err());
        assert!(validate_category("   ").is_err());
        assert!(validate_category(&"a".repeat(51)).is_err()); // 51文字
    }

    #[test]
    fn test_validate_description() {
        // 有効な説明
        assert!(validate_description(&None).is_ok());
        assert!(validate_description(&Some("短い説明".to_string())).is_ok());

        // 無効な説明
        assert!(validate_description(&Some("a".repeat(501))).is_err()); // 501文字
    }

    #[test]
    fn test_validate_https_url() {
        // 有効なURL
        assert!(validate_https_url("https://example.com").is_ok());
        assert!(validate_https_url("https://example.com/path").is_ok());

        // 無効なURL
        assert!(validate_https_url("http://example.com").is_err()); // HTTP
        assert!(validate_https_url("https://").is_err()); // ドメインなし
        assert!(validate_https_url("https://example").is_err()); // TLDなし
        assert!(validate_https_url("ftp://example.com").is_err()); // 異なるプロトコル
    }

    #[test]
    fn test_get_current_jst_timestamp() {
        let timestamp = get_current_jst_timestamp();

        // RFC3339形式であることを確認
        assert!(timestamp.contains('T'));
        assert!(timestamp.contains('+') || timestamp.contains('Z'));
    }

    #[test]
    fn test_get_today_date_jst() {
        let today = get_today_date_jst();

        // YYYY-MM-DD形式であることを確認
        assert_eq!(today.len(), 10);
        assert!(validate_date(&today).is_ok());
    }

    #[test]
    fn test_parse_date_to_jst_datetime() {
        // 有効な日付
        let result = parse_date_to_jst_datetime("2024-01-01");
        assert!(result.is_ok());

        // 無効な日付
        let result = parse_date_to_jst_datetime("invalid-date");
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_string() {
        assert_eq!(normalize_string("  テスト  "), "テスト");
        assert_eq!(normalize_string("テスト"), "テスト");
        assert_eq!(normalize_string("   "), "");
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(1000.0), "1000");
        assert_eq!(format_amount(1000.50), "1000.50");
        assert_eq!(format_amount(1234567.89), "1234567.89");
        assert_eq!(format_amount(0.01), "0.01");
    }
}
