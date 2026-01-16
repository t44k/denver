use std::path::Path;

use crate::error::DenverResult;
use crate::models::Environment;
use crate::scanner::parse_env_file;

/// Read an environment file (alias for scanner::parse_env_file)
pub fn read_env_file(path: &Path) -> DenverResult<Environment> {
    parse_env_file(path)
}
