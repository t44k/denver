//! Kubernetes YAML scanner for extracting environment variables from K8s manifests.

use std::path::Path;

use serde::Deserialize;

use crate::error::{DenverError, DenverResult};
use crate::models::{EnvVar, Environment, EnvironmentType};

/// Supported K8s workload kinds
const SUPPORTED_KINDS: &[&str] = &[
    "Deployment",
    "StatefulSet",
    "DaemonSet",
    "Job",
    "CronJob",
];

/// Top-level K8s manifest structure
#[derive(Debug, Deserialize)]
struct K8sManifest {
    kind: String,
    metadata: K8sMetadata,
    #[serde(default)]
    spec: Option<K8sSpec>,
}

#[derive(Debug, Deserialize)]
struct K8sMetadata {
    name: String,
    #[allow(dead_code)]
    #[serde(default)]
    namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct K8sSpec {
    #[serde(default)]
    template: Option<K8sPodTemplate>,
    /// For CronJobs which have a nested jobTemplate
    #[serde(rename = "jobTemplate", default)]
    job_template: Option<K8sJobTemplate>,
}

#[derive(Debug, Deserialize)]
struct K8sJobTemplate {
    #[serde(default)]
    spec: Option<K8sJobSpec>,
}

#[derive(Debug, Deserialize)]
struct K8sJobSpec {
    #[serde(default)]
    template: Option<K8sPodTemplate>,
}

#[derive(Debug, Deserialize)]
struct K8sPodTemplate {
    #[serde(default)]
    spec: Option<K8sPodSpec>,
}

#[derive(Debug, Deserialize)]
struct K8sPodSpec {
    #[serde(default)]
    containers: Vec<K8sContainer>,
}

#[derive(Debug, Deserialize)]
struct K8sContainer {
    name: String,
    #[serde(default)]
    env: Vec<K8sEnvVar>,
}

#[derive(Debug, Deserialize)]
struct K8sEnvVar {
    name: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(rename = "valueFrom", default)]
    value_from: Option<K8sValueFrom>,
}

#[derive(Debug, Deserialize)]
struct K8sValueFrom {
    #[serde(rename = "secretKeyRef", default)]
    secret_key_ref: Option<K8sKeyRef>,
    #[serde(rename = "configMapKeyRef", default)]
    config_map_key_ref: Option<K8sKeyRef>,
    #[serde(rename = "fieldRef", default)]
    field_ref: Option<K8sFieldRef>,
}

#[derive(Debug, Deserialize)]
struct K8sKeyRef {
    name: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct K8sFieldRef {
    #[serde(rename = "fieldPath")]
    field_path: String,
}

/// Scan a .k8s directory recursively for K8s manifests and extract environments
pub fn scan_k8s_directory(path: &Path) -> DenverResult<Vec<Environment>> {
    let mut environments = Vec::new();
    scan_k8s_directory_recursive(path, path, &mut environments)?;
    Ok(environments)
}

/// Recursively scan a directory for K8s manifests
fn scan_k8s_directory_recursive(
    root: &Path,
    current: &Path,
    environments: &mut Vec<Environment>,
) -> DenverResult<()> {
    let entries = std::fs::read_dir(current).map_err(|e| DenverError::DirectoryScan {
        path: current.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let file_path = entry.path();

        if file_path.is_dir() {
            // Recursively scan subdirectories
            if let Err(e) = scan_k8s_directory_recursive(root, &file_path, environments) {
                eprintln!("Warning: Could not scan K8s subdirectory {}: {}", file_path.display(), e);
            }
        } else if let Some(ext) = file_path.extension() {
            // Only process .yaml and .yml files
            if ext == "yaml" || ext == "yml" {
                // Calculate relative subdir from root
                let subdir = file_path
                    .parent()
                    .and_then(|p| p.strip_prefix(root).ok())
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| p.to_string_lossy().to_string());

                match parse_k8s_file(&file_path, subdir.as_deref()) {
                    Ok(mut envs) => environments.append(&mut envs),
                    Err(e) => {
                        eprintln!("Warning: Could not parse K8s file {}: {}", file_path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse a single K8s YAML file and extract environments
/// `subdir` is the relative path from the .k8s root (e.g., "overlays/production")
pub fn parse_k8s_file(path: &Path, subdir: Option<&str>) -> DenverResult<Vec<Environment>> {
    let content = std::fs::read_to_string(path).map_err(|e| DenverError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut environments = Vec::new();

    // Handle multi-document YAML (separated by ---)
    for doc in content.split("\n---") {
        let trimmed = doc.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match parse_k8s_document(trimmed, path, subdir) {
            Ok(mut envs) => environments.append(&mut envs),
            Err(_) => {
                // Skip documents that don't parse as K8s manifests
                continue;
            }
        }
    }

    Ok(environments)
}

/// Parse a single YAML document as a K8s manifest
fn parse_k8s_document(content: &str, path: &Path, subdir: Option<&str>) -> DenverResult<Vec<Environment>> {
    let manifest: K8sManifest = serde_yaml::from_str(content).map_err(|e| {
        DenverError::K8sParseError {
            path: path.to_path_buf(),
            message: e.to_string(),
        }
    })?;

    // Only process supported workload kinds
    if !SUPPORTED_KINDS.contains(&manifest.kind.as_str()) {
        return Ok(Vec::new());
    }

    let containers = extract_containers(&manifest);
    if containers.is_empty() {
        return Ok(Vec::new());
    }

    let mut environments = Vec::new();
    let has_multiple_containers = containers.len() > 1;

    for container in containers {
        if container.env.is_empty() {
            continue;
        }

        let env_type = EnvironmentType::Kubernetes {
            subdir: subdir.map(String::from),
            resource_name: manifest.metadata.name.clone(),
            container_name: if has_multiple_containers {
                Some(container.name.clone())
            } else {
                None
            },
        };

        let mut env = Environment::new(env_type, path.to_path_buf());

        for (idx, k8s_var) in container.env.iter().enumerate() {
            let value = resolve_env_value(k8s_var);
            let mut var = EnvVar::new(k8s_var.name.clone(), value);
            var.line_number = Some(idx + 1); // Use index as pseudo line number
            env.variables.insert(k8s_var.name.clone(), var);
        }

        environments.push(env);
    }

    Ok(environments)
}

/// Extract containers from a K8s manifest based on its kind
fn extract_containers(manifest: &K8sManifest) -> Vec<&K8sContainer> {
    let mut containers = Vec::new();

    if let Some(spec) = &manifest.spec {
        // For CronJobs, containers are in spec.jobTemplate.spec.template.spec.containers
        if manifest.kind == "CronJob" {
            if let Some(job_template) = &spec.job_template {
                if let Some(job_spec) = &job_template.spec {
                    if let Some(template) = &job_spec.template {
                        if let Some(pod_spec) = &template.spec {
                            containers.extend(pod_spec.containers.iter());
                        }
                    }
                }
            }
        } else {
            // For Deployment, StatefulSet, DaemonSet, Job:
            // containers are in spec.template.spec.containers
            if let Some(template) = &spec.template {
                if let Some(pod_spec) = &template.spec {
                    containers.extend(pod_spec.containers.iter());
                }
            }
        }
    }

    containers
}

/// Resolve the value of a K8s env var, handling valueFrom references
fn resolve_env_value(k8s_var: &K8sEnvVar) -> String {
    // If there's a direct value, use it
    if let Some(value) = &k8s_var.value {
        return value.clone();
    }

    // If there's a valueFrom, create a descriptive placeholder
    if let Some(value_from) = &k8s_var.value_from {
        if let Some(secret_ref) = &value_from.secret_key_ref {
            return format!("<from:secret/{}.{}>", secret_ref.name, secret_ref.key);
        }
        if let Some(config_map_ref) = &value_from.config_map_key_ref {
            return format!("<from:configmap/{}.{}>", config_map_ref.name, config_map_ref.key);
        }
        if let Some(field_ref) = &value_from.field_ref {
            return format!("<from:field/{}>", field_ref.field_path);
        }
    }

    // No value specified
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deployment() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-app
  namespace: default
spec:
  template:
    spec:
      containers:
        - name: main
          env:
            - name: PORT
              value: "8080"
            - name: SECRET
              value: "abc123"
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), None).unwrap();
        assert_eq!(envs.len(), 1);

        let env = &envs[0];
        assert_eq!(env.variables.len(), 2);
        assert_eq!(env.get_value("PORT"), Some("8080"));
        assert_eq!(env.get_value("SECRET"), Some("abc123"));
        assert!(env.is_readonly);

        // Check display name without subdir
        assert_eq!(env.env_type.display_name(), "k8s:test-app");
    }

    #[test]
    fn test_parse_deployment_with_subdir() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
spec:
  template:
    spec:
      containers:
        - name: main
          env:
            - name: PORT
              value: "8080"
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), Some("overlays/production")).unwrap();
        assert_eq!(envs.len(), 1);

        let env = &envs[0];
        // Check display name with subdir (colons as separators)
        assert_eq!(env.env_type.display_name(), "k8s:overlays:production:api-server");
    }

    #[test]
    fn test_parse_multiple_containers() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: multi-container
spec:
  template:
    spec:
      containers:
        - name: main
          env:
            - name: PORT
              value: "80"
        - name: sidecar
          env:
            - name: LOG_LEVEL
              value: "debug"
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), None).unwrap();
        assert_eq!(envs.len(), 2);

        // Check that container names are included when there are multiple containers
        match &envs[0].env_type {
            EnvironmentType::Kubernetes { container_name, .. } => {
                assert!(container_name.is_some());
            }
            _ => panic!("Expected Kubernetes environment type"),
        }

        // Check display names include container
        assert_eq!(envs[0].env_type.display_name(), "k8s:multi-container:main");
        assert_eq!(envs[1].env_type.display_name(), "k8s:multi-container:sidecar");
    }

    #[test]
    fn test_parse_value_from_secret() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secret-app
spec:
  template:
    spec:
      containers:
        - name: main
          env:
            - name: DB_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: db-secrets
                  key: password
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), None).unwrap();
        assert_eq!(envs.len(), 1);

        let env = &envs[0];
        assert_eq!(env.get_value("DB_PASSWORD"), Some("<from:secret/db-secrets.password>"));
    }

    #[test]
    fn test_skip_unsupported_kinds() {
        let yaml = r#"
apiVersion: v1
kind: Service
metadata:
  name: test-service
spec:
  ports:
    - port: 80
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), None).unwrap();
        assert!(envs.is_empty());
    }

    #[test]
    fn test_parse_cronjob() {
        let yaml = r#"
apiVersion: batch/v1
kind: CronJob
metadata:
  name: backup-job
spec:
  schedule: "0 0 * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: backup
              env:
                - name: BACKUP_PATH
                  value: "/data"
"#;
        let envs = parse_k8s_document(yaml, Path::new("test.yaml"), None).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].get_value("BACKUP_PATH"), Some("/data"));
    }
}
