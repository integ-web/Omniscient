use serde::{Deserialize, Serialize};

/// Data originating from LLMs, User Input, or Web Scraping.
/// It MUST NOT be stored in memory graphs or databases until verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustedValue<T> {
    data: T,
}

impl<T> UntrustedValue<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Data that has passed PrincipalChecker and is safe for system mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedAction<T> {
    data: T,
}

impl<T> TrustedAction<T> {
    pub fn into_inner(self) -> T {
        self.data
    }
}

/// Security Policy Gate (Mimicking Crust's brain::taint)
pub struct PrincipalChecker;

impl PrincipalChecker {
    /// Validates an untrusted value and promotes it to a TrustedAction if safe.
    ///
    /// For the MVP, we just apply generic sanity checks to strings.
    pub fn verify_string(value: UntrustedValue<String>) -> Option<TrustedAction<String>> {
        let raw = value.data;

        // Simple security logic to prevent path traversal or raw executable injection
        if raw.contains("<script>") || raw.contains("../") {
            return None;
        }

        Some(TrustedAction { data: raw })
    }

    pub fn verify_findings<T: Clone>(value: UntrustedValue<T>) -> Option<TrustedAction<T>> {
        // In a full implementation, you would introspect the T (Finding) struct and validate all fields.
        Some(TrustedAction { data: value.data })
    }
}
