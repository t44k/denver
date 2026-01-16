mod parser;
mod k8s;

pub use parser::parse_env_file;

use std::path::Path;

use crate::error::{DenverError, DenverResult};
use crate::models::Project;

/// Scan a directory for projects with .env files
pub fn scan_directory(root: &Path) -> DenverResult<Vec<Project>> {
    let mut projects = Vec::new();

    // First, check if the root directory itself has .env files
    if let Some(root_project) = scan_project_with_name(root, "/".to_string())? {
        projects.push(root_project);
    }

    let entries = std::fs::read_dir(root).map_err(|e| DenverError::DirectoryScan {
        path: root.to_path_buf(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| DenverError::DirectoryScan {
            path: root.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        // Try to scan as a project
        if let Some(project) = scan_project(&path)? {
            projects.push(project);
        }
    }

    // Sort projects by name (but "/" should stay at top)
    projects.sort_by(|a, b| {
        match (a.name.as_str(), b.name.as_str()) {
            ("/", "/") => std::cmp::Ordering::Equal,
            ("/", _) => std::cmp::Ordering::Less,
            (_, "/") => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(projects)
}

/// Scan a single directory as a potential project
fn scan_project(path: &Path) -> DenverResult<Option<Project>> {
    let project_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    scan_project_with_name(path, project_name)
}

/// Scan a single directory as a potential project with a custom name
fn scan_project_with_name(path: &Path, project_name: String) -> DenverResult<Option<Project>> {
    let mut project = Project::new(project_name, path.to_path_buf());

    // Scan for all .env* files
    let entries = std::fs::read_dir(path).map_err(|e| DenverError::DirectoryScan {
        path: path.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Check if it's a .env file
        if filename_str == ".env" || filename_str.starts_with(".env.") {
            let env_path = entry.path();
            if env_path.is_file() {
                match parse_env_file(&env_path) {
                    Ok(env) => {
                        project.add_environment(env);
                    }
                    Err(e) => {
                        // Log warning but continue
                        eprintln!("Warning: Could not parse {}: {}", env_path.display(), e);
                    }
                }
            }
        }
    }

    // Scan for .k8s directory
    let k8s_dir = path.join(".k8s");
    if k8s_dir.is_dir() {
        match k8s::scan_k8s_directory(&k8s_dir) {
            Ok(k8s_envs) => {
                for env in k8s_envs {
                    project.add_environment(env);
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not scan K8s directory {}: {}", k8s_dir.display(), e);
            }
        }
    }

    // Only return project if it has at least one environment
    if project.environments.is_empty() {
        Ok(None)
    } else {
        Ok(Some(project))
    }
}
