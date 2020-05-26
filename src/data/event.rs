////////////////////////////////////////////////////////////////////////////////
// DATA STRUCTURES
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct Event {
    pub content_type: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

////////////////////////////////////////////////////////////////////////////////
// TRAIT FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl Default for Event {
    fn default() -> Self {
        Self {
            content_type: String::default(),
            content: String::default(),
            metadata: serde_json::json!({}),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// METHOD FUNCTIONS
////////////////////////////////////////////////////////////////////////////////

impl Event {
    pub fn new(content_type: &str, content: &str, metadata: serde_json::Value) -> Self {
        Self {
            content_type: content_type.to_owned(),
            content: content.to_owned(),
            metadata: metadata.to_owned(),
        }
    }
}
