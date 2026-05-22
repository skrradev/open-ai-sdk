use std::{borrow::Borrow, fmt, ops::Deref};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
/// String-backed model identifier.
///
/// `ModelId` intentionally accepts arbitrary strings so callers can use new
/// OpenAI model IDs immediately without waiting for a crate release. Convenience
/// constants are available in [`crate::models::ids`].
///
/// # Example
///
/// ```
/// use open_ai_sdk::ModelId;
///
/// let current = ModelId::from("gpt-4o");
/// let future = ModelId::from("gpt-6");
/// assert_eq!(current.as_str(), "gpt-4o");
/// assert_eq!(future.as_str(), "gpt-6");
/// ```
pub struct ModelId(String);

impl ModelId {
    /// Creates a model ID from any string-like value.
    pub fn new(model: impl Into<String>) -> Self {
        Self(model.into())
    }

    /// Returns the model ID as `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the model ID and returns the underlying string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<&str> for ModelId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ModelId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&String> for ModelId {
    fn from(value: &String) -> Self {
        Self::new(value.clone())
    }
}

impl AsRef<str> for ModelId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ModelId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for ModelId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
