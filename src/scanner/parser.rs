use std::path::Path;

use crate::error::{DenverError, DenverResult};
use crate::models::{EnvVar, Environment, EnvironmentType};

/// Parse a .env file into an Environment struct
pub fn parse_env_file(path: &Path) -> DenverResult<Environment> {
    let content = std::fs::read_to_string(path).map_err(|e| DenverError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or(".env");

    let env_type = EnvironmentType::from_filename(filename);
    let mut env = Environment::new(env_type, path.to_path_buf());
    let mut current_comment: Option<String> = None;
    let mut current_section: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Empty lines end the current section context
        if trimmed.is_empty() {
            current_comment = None;
            current_section = None;
            continue;
        }

        // Check for section marker: # [Section Name] or // [Section Name]
        if let Some(section) = parse_section_marker(trimmed) {
            current_section = Some(section);
            current_comment = None;
            continue;
        }

        // Capture comments (# or // style)
        if trimmed.starts_with('#') {
            current_comment = Some(trimmed[1..].trim().to_string());
            continue;
        }
        if trimmed.starts_with("//") {
            current_comment = Some(trimmed[2..].trim().to_string());
            continue;
        }

        // Parse key=value
        if let Some((key, value)) = parse_line(trimmed) {
            let mut var = EnvVar::new(key.clone(), value).with_line_number(line_num + 1);

            if let Some(comment) = current_comment.take() {
                var = var.with_comment(comment);
            }

            if let Some(ref section) = current_section {
                var = var.with_section(section.clone());
            }

            // Check if this key already exists - if so, mark the old one as duplicated
            if let Some(mut old_var) = env.variables.remove(&key) {
                old_var.is_duplicated = true;
                env.duplicated_vars.push(old_var);
            }

            env.variables.insert(key, var);
        }
    }

    Ok(env)
}

/// Parse a section marker in the format: # [Section Name] or // [Section Name]
fn parse_section_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Try # [Section] format first (preferred for .env files)
    let rest = if trimmed.starts_with('#') {
        Some(trimmed[1..].trim())
    } else if trimmed.starts_with("//") {
        Some(trimmed[2..].trim())
    } else {
        None
    };

    if let Some(rest) = rest {
        if rest.starts_with('[') && rest.ends_with(']') {
            let section_name = rest[1..rest.len() - 1].trim().to_string();
            if !section_name.is_empty() {
                return Some(section_name);
            }
        }
    }
    None
}

/// Parse a single line into key-value pair
fn parse_line(line: &str) -> Option<(String, String)> {
    // Find the first = that's not inside quotes
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut equals_pos = None;

    for (i, c) in line.char_indices() {
        match c {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            }
            c if c == quote_char && in_quotes => {
                in_quotes = false;
            }
            '=' if !in_quotes && equals_pos.is_none() => {
                equals_pos = Some(i);
            }
            _ => {}
        }
    }

    let pos = equals_pos?;
    let key = line[..pos].trim().to_string();
    let value = line[pos + 1..].trim();

    // Remove surrounding quotes if present
    let value = if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        if value.len() >= 2 {
            value[1..value.len() - 1].to_string()
        } else {
            String::new()
        }
    } else {
        value.to_string()
    };

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_line() {
        assert_eq!(
            parse_line("KEY=value"),
            Some(("KEY".to_string(), "value".to_string()))
        );
    }

    #[test]
    fn test_parse_quoted_value() {
        assert_eq!(
            parse_line("KEY=\"value with spaces\""),
            Some(("KEY".to_string(), "value with spaces".to_string()))
        );
    }

    #[test]
    fn test_parse_single_quoted_value() {
        assert_eq!(
            parse_line("KEY='value'"),
            Some(("KEY".to_string(), "value".to_string()))
        );
    }

    #[test]
    fn test_parse_empty_value() {
        assert_eq!(
            parse_line("KEY="),
            Some(("KEY".to_string(), String::new()))
        );
    }

    #[test]
    fn test_parse_value_with_equals() {
        assert_eq!(
            parse_line("URL=postgres://user:pass@host/db?sslmode=require"),
            Some((
                "URL".to_string(),
                "postgres://user:pass@host/db?sslmode=require".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_section_marker() {
        // # [Section] format (preferred for .env)
        assert_eq!(
            parse_section_marker("# [Session Settings]"),
            Some("Session Settings".to_string())
        );
        assert_eq!(
            parse_section_marker("#[Admin API]"),
            Some("Admin API".to_string())
        );
        assert_eq!(
            parse_section_marker("  # [Features]  "),
            Some("Features".to_string())
        );
        // // [Section] format (also supported)
        assert_eq!(
            parse_section_marker("// [Session Settings]"),
            Some("Session Settings".to_string())
        );
        assert_eq!(
            parse_section_marker("//[Admin API]"),
            Some("Admin API".to_string())
        );
        // Not section markers
        assert_eq!(parse_section_marker("# Section"), None);
        assert_eq!(parse_section_marker("// Not a section"), None);
        assert_eq!(parse_section_marker("# []"), None);
        assert_eq!(parse_section_marker("// []"), None);
    }
}
