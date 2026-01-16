use std::io::Write;
use std::path::Path;

use crate::error::{DenverError, DenverResult};
use crate::models::{Environment, Project};

/// Write environment back to file with optional backup
pub fn write_env_file(env: &Environment, backup: bool) -> DenverResult<()> {
    let path = &env.path;

    // Create backup if requested
    if backup && path.exists() {
        let backup_path = path.with_extension("env.bak");
        std::fs::copy(path, &backup_path).map_err(|e| DenverError::FileWrite {
            path: backup_path,
            source: e,
        })?;
    }

    // Write to temp file first (atomic write pattern)
    let temp_path = path.with_extension("env.tmp");
    let mut file = std::fs::File::create(&temp_path).map_err(|e| DenverError::FileWrite {
        path: temp_path.clone(),
        source: e,
    })?;

    // Sort variables by original line number to preserve order
    let mut vars: Vec<_> = env.variables.values().collect();
    vars.sort_by_key(|v| v.line_number.unwrap_or(usize::MAX));

    let mut current_section: Option<&str> = None;
    let mut is_first = true;

    for var in vars {
        // Write section header if section changed
        let var_section = var.section.as_deref();
        if var_section != current_section {
            // Add blank line before new section (except at file start)
            if !is_first && current_section.is_some() {
                writeln!(file).map_err(|e| DenverError::FileWrite {
                    path: temp_path.clone(),
                    source: e,
                })?;
            }

            // Write section header if this section has a name
            if let Some(section_name) = var_section {
                writeln!(file, "# [{}]", section_name).map_err(|e| DenverError::FileWrite {
                    path: temp_path.clone(),
                    source: e,
                })?;
            }
            current_section = var_section;
        }

        // Write comment if present
        if let Some(ref comment) = var.comment {
            writeln!(file, "# {}", comment).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        }

        // Determine if value needs quoting
        let needs_quotes = var.value.contains(' ')
            || var.value.contains('#')
            || var.value.contains('"')
            || var.value.contains('\'')
            || var.value.contains('=')
            || var.value.is_empty();

        if needs_quotes {
            // Use double quotes, escape internal quotes
            let escaped = var.value.replace('\\', "\\\\").replace('"', "\\\"");
            writeln!(file, "{}=\"{}\"", var.key, escaped).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        } else {
            writeln!(file, "{}={}", var.key, var.value).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        }

        is_first = false;
    }

    // Atomic rename
    std::fs::rename(&temp_path, path).map_err(|e| DenverError::FileWrite {
        path: path.clone(),
        source: e,
    })?;

    Ok(())
}

/// Save all dirty environments in a project
pub fn save_project(project: &mut Project, backup: bool) -> DenverResult<usize> {
    let mut saved_count = 0;

    for env in project.environments.values_mut() {
        if env.is_dirty {
            write_env_file(env, backup)?;
            env.mark_clean();
            saved_count += 1;
        }
    }

    Ok(saved_count)
}

/// Write environment to a specific output file path
pub fn write_env_to_path(env: &Environment, output_path: &Path) -> DenverResult<()> {
    // Write to temp file first (atomic write pattern)
    let temp_path = output_path.with_extension("tmp");
    let mut file = std::fs::File::create(&temp_path).map_err(|e| DenverError::FileWrite {
        path: temp_path.clone(),
        source: e,
    })?;

    // Sort variables by original line number to preserve order
    let mut vars: Vec<_> = env.variables.values().collect();
    vars.sort_by_key(|v| v.line_number.unwrap_or(usize::MAX));

    let mut current_section: Option<&str> = None;
    let mut is_first = true;

    for var in vars {
        // Write section header if section changed
        let var_section = var.section.as_deref();
        if var_section != current_section {
            // Add blank line before new section (except at file start)
            if !is_first && current_section.is_some() {
                writeln!(file).map_err(|e| DenverError::FileWrite {
                    path: temp_path.clone(),
                    source: e,
                })?;
            }

            // Write section header if this section has a name
            if let Some(section_name) = var_section {
                writeln!(file, "# [{}]", section_name).map_err(|e| DenverError::FileWrite {
                    path: temp_path.clone(),
                    source: e,
                })?;
            }
            current_section = var_section;
        }

        // Write comment if present
        if let Some(ref comment) = var.comment {
            writeln!(file, "# {}", comment).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        }

        // Determine if value needs quoting
        let needs_quotes = var.value.contains(' ')
            || var.value.contains('#')
            || var.value.contains('"')
            || var.value.contains('\'')
            || var.value.contains('=')
            || var.value.is_empty();

        if needs_quotes {
            // Use double quotes, escape internal quotes
            let escaped = var.value.replace('\\', "\\\\").replace('"', "\\\"");
            writeln!(file, "{}=\"{}\"", var.key, escaped).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        } else {
            writeln!(file, "{}={}", var.key, var.value).map_err(|e| DenverError::FileWrite {
                path: temp_path.clone(),
                source: e,
            })?;
        }

        is_first = false;
    }

    // Atomic rename
    std::fs::rename(&temp_path, output_path).map_err(|e| DenverError::FileWrite {
        path: output_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Save project's default environment to a specific output filename in the project's directory
pub fn save_project_to_output(project: &Project, output_filename: &str) -> DenverResult<()> {
    use crate::models::EnvironmentType;

    // Construct full path: project directory + output filename
    let output_path = project.path.join(output_filename);

    // Get the default environment (.env)
    if let Some(env) = project.environments.get(&EnvironmentType::Default) {
        write_env_to_path(env, &output_path)?;
    }

    Ok(())
}
