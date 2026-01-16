/// Represents a single environment variable key-value pair
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub line_number: Option<usize>,
    pub comment: Option<String>,
    pub section: Option<String>,
    /// True if this key appears earlier in the file and is overwritten by a later duplicate
    pub is_duplicated: bool,
}

impl EnvVar {
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            line_number: None,
            comment: None,
            section: None,
            is_duplicated: false,
        }
    }

    pub fn with_line_number(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub fn with_section(mut self, section: String) -> Self {
        self.section = Some(section);
        self
    }
}
