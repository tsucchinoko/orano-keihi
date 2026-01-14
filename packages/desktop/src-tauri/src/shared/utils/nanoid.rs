use nanoid::nanoid;

/// ユーザーID用のnanoIdを生成する
///
/// # 戻り値
/// 21文字のURL-safeなnanoId
///
/// # 特性
/// - 文字セット: A-Za-z0-9_- (64文字)
/// - 長さ: 21文字
/// - 衝突確率: 1兆個のIDで1%未満
pub fn generate_user_id() -> String {
    nanoid!()
}

/// カスタム長のnanoIdを生成する（テスト用）
///
/// # 引数
/// * `length` - 生成するIDの長さ
///
/// # 戻り値
/// 指定された長さのnanoId
pub fn generate_user_id_with_length(length: usize) -> String {
    nanoid!(length)
}

/// nanoIdが有効な形式かどうかを検証する
///
/// # 引数
/// * `id` - 検証するID文字列
///
/// # 戻り値
/// 有効な場合はtrue、無効な場合はfalse
///
/// # 検証条件
/// - 長さが21文字
/// - URL-safe文字（A-Za-z0-9_-）のみを含む
pub fn is_valid_nanoid(id: &str) -> bool {
    id.len() == 21
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_id_length() {
        let id = generate_user_id();
        assert_eq!(id.len(), 21);
    }

    #[test]
    fn test_generate_user_id_uniqueness() {
        let id1 = generate_user_id();
        let id2 = generate_user_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_user_id_url_safe() {
        let id = generate_user_id();
        // URL-safeな文字のみを含むことを確認
        assert!(id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn test_generate_user_id_with_custom_length() {
        let id = generate_user_id_with_length(10);
        assert_eq!(id.len(), 10);
    }

    #[test]
    fn test_is_valid_nanoid() {
        // 有効なnanoId
        let valid_id = generate_user_id();
        assert!(is_valid_nanoid(&valid_id));

        // 有効なnanoId（数字のみでも21文字ならOK）
        assert!(is_valid_nanoid("123456789012345678901"));

        // 無効なnanoId（長さが異なる）
        assert!(!is_valid_nanoid("short"));
        assert!(!is_valid_nanoid(
            "this_is_way_too_long_to_be_a_valid_nanoid"
        ));

        // 無効なnanoId（無効な文字を含む）
        assert!(!is_valid_nanoid("invalid@characters!!"));
        assert!(!is_valid_nanoid("123456789012345678@01")); // 21文字だが@を含む

        // 無効なnanoId（スペースを含む）
        assert!(!is_valid_nanoid("has space in it 12345"));
    }
}
