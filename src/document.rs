//! Explicit public text, never a path/URL fetcher or a credential store.
use crate::config_lexical::validate_text;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicDocument {
    pub classification: String,
    pub media_type: String,
    pub source_url: String,
    pub sha256: String,
    pub license: String,
    pub attribution: String,
    // Catalog/tool metadata never duplicates complete document bodies.
    #[serde(skip_serializing)]
    pub text: String,
}
impl PublicDocument {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.classification != "public"
            || !matches!(self.media_type.as_str(), "text/plain" | "text/markdown")
            || self.text.trim().is_empty()
            || self.text.len() > 16384
            || self
                .text
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
            || !self.source_url.starts_with("https://")
            || self.source_url.len() > 512
            || self
                .source_url
                .bytes()
                .any(|c| c.is_ascii_whitespace() || matches!(c, b'@' | b'?' | b'#'))
            || self.sha256 != hex::encode(Sha256::digest(self.text.as_bytes()))
        {
            return Err("invalid or unverified public document".into());
        }
        validate_text(&self.license, 160)?;
        validate_text(&self.attribution, 160)?;
        Ok(())
    }
}
