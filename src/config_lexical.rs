//! Existing metadata identifier/text and marker admission rules.

pub(crate) fn validate_id(value: &str) -> Result<(), String> {
    let valid = (1..=63).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    if valid {
        Ok(())
    } else {
        Err("public IDs must be lowercase kebab-case identifiers".into())
    }
}

pub(crate) fn validate_text(value: &str, max: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err("public text is empty, oversized, or contains control characters".into());
    }
    Ok(())
}

pub(crate) fn has_sensitive_marker(input: &str) -> bool {
    let lowered = input.to_ascii_lowercase();
    [
        "password",
        "secret",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "authorization",
        "bearer ",
        "private_key",
        "-----begin",
        "connection_string",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}
