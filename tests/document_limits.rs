use sha2::{Digest, Sha256};
use zixcel_mcp_gateway::Config;
fn with_text(text: &str, count: usize) -> String {
    let mut config: toml::Value =
        toml::from_str(include_str!("../examples/documents.toml")).unwrap();
    let original = config["resource"].as_array().unwrap()[0].clone();
    let resources = config["resource"].as_array_mut().unwrap();
    resources.clear();
    for index in 0..count {
        let mut resource = original.clone();
        resource["id"] = toml::Value::String(format!("guidance-{index}"));
        resource["document"]["text"] = toml::Value::String(text.into());
        resource["document"]["sha256"] =
            toml::Value::String(hex::encode(Sha256::digest(text.as_bytes())));
        resources.push(resource);
    }
    toml::to_string(&config).unwrap()
}
#[test]
fn correct_digest_does_not_bypass_individual_utf8_or_total_byte_limits() {
    assert!(Config::from_toml(&with_text(&"x".repeat(16384), 2)).is_ok());
    assert!(Config::from_toml(&with_text(&"x".repeat(16385), 1)).is_err());
    assert!(Config::from_toml(&with_text(&"あ".repeat(6000), 1)).is_err());
    let error = Config::from_toml(&with_text(&"x".repeat(12000), 3)).unwrap_err();
    assert!(error.contains("aggregate"));
}
#[test]
fn blank_and_control_text_and_extra_document_fields_are_not_public_content() {
    for text in ["", "   ", "line\0content"] {
        assert!(Config::from_toml(&with_text(text, 1)).is_err());
    }
    let source = format!(
        "{}\npath = \"not-a-loader\"",
        with_text("valid public text", 1)
    );
    assert!(Config::from_toml(&source).is_err());
}
