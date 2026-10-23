//! Automatic secret & credential masking for AI prompts and external data transmissions.
//! Prevents accidental leakage of API keys, private keys, access tokens, and passwords.

use regex::Regex;
use std::sync::LazyLock;

static PRIVATE_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)-----BEGIN [A-Z0-9_\- ]*PRIVATE KEY-----.*?-----END [A-Z0-9_\- ]*PRIVATE KEY-----",
    )
    .expect("valid private key regex")
});

static GITHUB_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(gh[pousr]_[A-Za-z0-9_]{36,}|github_pat_[A-Za-z0-9_]{36,})\b")
        .expect("valid github token regex")
});

static GOOGLE_API_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bAIza[0-9A-Za-z\-_]{35}\b").expect("valid google api key regex")
});

static OPENAI_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsk-[A-Za-z0-9_\-]{20,}\b").expect("valid openai key regex"));

static ANTHROPIC_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bsk-ant-[A-Za-z0-9_\-]{20,}\b").expect("valid anthropic key regex")
});

static AWS_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(AKIA|ASIA)[0-9A-Z]{16}\b").expect("valid aws key regex"));

static SLACK_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bxox[baprs]-[0-9a-zA-Z\-]{10,}\b").expect("valid slack token regex")
});

static JWT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[A-Za-z0-9\-_=]+\.eyJ[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_.+/=]+\b")
        .expect("valid jwt regex")
});

static ASSIGNMENT_SECRET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(\b[A-Za-z0-9_]*(?:api[_-]?key|access[_-]?token|auth[_-]?token|secret[_-]?key|client[_-]?secret|secret|password|passwd)[A-Za-z0-9_]*\s*[:=]\s*["']?)([A-Za-z0-9_\-./+]{8,})(["']?)"#)
        .expect("valid assignment secret regex")
});

/// Mask sensitive tokens, API credentials, and private keys in diff or text before sending to AI.
pub fn mask_secrets(input: &str) -> String {
    let mut text = input.to_string();

    // 1. Mask multiline private keys
    text = PRIVATE_KEY_REGEX
        .replace_all(&text, "[REDACTED_PRIVATE_KEY]")
        .to_string();

    // 2. Mask platform-specific tokens
    text = GITHUB_TOKEN_REGEX
        .replace_all(&text, "[REDACTED_GITHUB_TOKEN]")
        .to_string();

    text = GOOGLE_API_KEY_REGEX
        .replace_all(&text, "[REDACTED_GOOGLE_API_KEY]")
        .to_string();

    text = OPENAI_KEY_REGEX
        .replace_all(&text, "[REDACTED_OPENAI_KEY]")
        .to_string();

    text = ANTHROPIC_KEY_REGEX
        .replace_all(&text, "[REDACTED_ANTHROPIC_KEY]")
        .to_string();

    text = AWS_KEY_REGEX
        .replace_all(&text, "[REDACTED_AWS_KEY]")
        .to_string();

    text = SLACK_TOKEN_REGEX
        .replace_all(&text, "[REDACTED_SLACK_TOKEN]")
        .to_string();

    text = JWT_REGEX
        .replace_all(&text, "[REDACTED_JWT_TOKEN]")
        .to_string();

    // 3. Mask key-value assignments (e.g. API_KEY="secret_value")
    text = ASSIGNMENT_SECRET_REGEX
        .replace_all(&text, "${1}[REDACTED_SECRET]${3}")
        .to_string();

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_private_keys() {
        let diff = r#"
+-----BEGIN RSA PRIVATE KEY-----
+MIIEowIBAAKCAQEA0Y1m9...
+fake_key_data_here...
+-----END RSA PRIVATE KEY-----
"#;
        let masked = mask_secrets(diff);
        assert!(!masked.contains("fake_key_data_here"));
        assert!(masked.contains("[REDACTED_PRIVATE_KEY]"));
    }

    #[test]
    fn test_mask_github_tokens() {
        let text = "token = 'ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'";
        let masked = mask_secrets(text);
        assert!(!masked.contains("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"));
        assert!(masked.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn test_mask_google_and_openai_keys() {
        let text = "google: AIzaSyA1234567890abcdefghijklmnopqrstuv, openai: sk-proj-1234567890abcdefghijklmn";
        let masked = mask_secrets(text);
        assert!(!masked.contains("AIzaSyA1234567890abcdefghijklmnopqrstuv"));
        assert!(!masked.contains("sk-proj-1234567890abcdefghijklmn"));
    }

    #[test]
    fn test_mask_assignment_secret() {
        let text = r#"export DB_PASSWORD="my_super_secret_db_pass_1234""#;
        let masked = mask_secrets(text);
        assert!(!masked.contains("my_super_secret_db_pass_1234"));
        assert!(masked.contains(r#"export DB_PASSWORD="[REDACTED_SECRET]""#));
    }
}
