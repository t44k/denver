mod env_var;
mod environment;
mod project;

pub use env_var::EnvVar;
pub use environment::{Environment, EnvironmentType};
pub use project::Project;

/// A displayable item in the project detail view - either a section header or a key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayItem {
    /// A section header (the section name)
    Section(String),
    /// A variable key
    Key(String),
    /// A duplicated key (earlier occurrence that was overwritten) - contains key, value, and line number
    DuplicatedKey(String, String, usize),
    /// An inactive key (exists in other envs but not in target .env)
    InactiveKey(String),
}
