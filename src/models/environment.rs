use std::collections::BTreeMap;
use std::path::PathBuf;

use super::EnvVar;

/// Represents an environment type
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EnvironmentType {
    /// The main .env file
    Default,
    /// Any .env.* file (e.g., .env.dev, .env.staging, .env.live)
    Named(String),
    /// Kubernetes manifest environment variables
    Kubernetes {
        /// Subdirectory path under .k8s (e.g., "overlays/production")
        subdir: Option<String>,
        resource_name: String,
        container_name: Option<String>,
    },
}

impl EnvironmentType {
    pub fn from_filename(filename: &str) -> Self {
        if filename == ".env" {
            Self::Default
        } else if let Some(suffix) = filename.strip_prefix(".env.") {
            Self::Named(suffix.to_string())
        } else {
            Self::Named(filename.to_string())
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Default => ".env".to_string(),
            Self::Named(name) => name.clone(),
            Self::Kubernetes { subdir, resource_name, container_name } => {
                // Format: k8s:subdir:resource or k8s:resource (colons as separators)
                let mut parts = vec!["k8s".to_string()];
                if let Some(sd) = subdir {
                    // Convert path separators to colons
                    parts.push(sd.replace(['/', '\\'], ":"));
                }
                parts.push(resource_name.clone());
                if let Some(container) = container_name {
                    parts.push(container.clone());
                }
                parts.join(":")
            }
        }
    }

    pub fn filename(&self) -> String {
        // For K8s, filename() returns the same as display_name()
        self.display_name()
    }

    /// Returns sort order priority (Default first, then Named alphabetical, then K8s)
    pub fn sort_key(&self) -> (u8, String, String, String) {
        match self {
            Self::Default => (0, String::new(), String::new(), String::new()),
            Self::Named(name) => (1, name.clone(), String::new(), String::new()),
            Self::Kubernetes { subdir, resource_name, container_name } => {
                (
                    2,
                    subdir.clone().unwrap_or_default(),
                    resource_name.clone(),
                    container_name.clone().unwrap_or_default(),
                )
            }
        }
    }

    /// Check if this is a Kubernetes environment type
    pub fn is_kubernetes(&self) -> bool {
        matches!(self, Self::Kubernetes { .. })
    }
}

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Represents an environment configuration file
#[derive(Debug, Clone)]
pub struct Environment {
    pub env_type: EnvironmentType,
    pub path: PathBuf,
    pub variables: BTreeMap<String, EnvVar>,
    /// Stores earlier occurrences of keys that were overwritten by duplicates
    pub duplicated_vars: Vec<EnvVar>,
    pub is_dirty: bool,
    /// K8s environments are read-only
    pub is_readonly: bool,
}

impl Environment {
    pub fn new(env_type: EnvironmentType, path: PathBuf) -> Self {
        let is_readonly = env_type.is_kubernetes();
        Self {
            env_type,
            path,
            variables: BTreeMap::new(),
            duplicated_vars: Vec::new(),
            is_dirty: false,
            is_readonly,
        }
    }

    pub fn get(&self, key: &str) -> Option<&EnvVar> {
        self.variables.get(key)
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|v| v.value.as_str())
    }

    pub fn set(&mut self, key: String, value: String) {
        if let Some(var) = self.variables.get_mut(&key) {
            if var.value != value {
                var.value = value;
                self.is_dirty = true;
            }
        } else {
            self.variables.insert(key.clone(), EnvVar::new(key, value));
            self.is_dirty = true;
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<EnvVar> {
        let result = self.variables.remove(key);
        if result.is_some() {
            self.is_dirty = true;
        }
        result
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Recalculate which duplicate is the "winner" based on line numbers.
    /// The key with the highest line number goes to variables (it's the active one).
    /// All other occurrences go to duplicated_vars.
    pub fn recalculate_duplicates(&mut self) {
        // Collect all occurrences of each key
        let mut all_occurrences: std::collections::BTreeMap<String, Vec<EnvVar>> = std::collections::BTreeMap::new();

        // Add variables - take ownership by collecting keys first
        let keys: Vec<String> = self.variables.keys().cloned().collect();
        for key in keys {
            if let Some(var) = self.variables.remove(&key) {
                all_occurrences.entry(key).or_default().push(var);
            }
        }

        // Add duplicated_vars
        for var in std::mem::take(&mut self.duplicated_vars) {
            all_occurrences.entry(var.key.clone()).or_default().push(var);
        }

        // For each key, put the one with highest line number in variables, rest in duplicated_vars
        for (key, mut occurrences) in all_occurrences {
            if occurrences.len() == 1 {
                // No duplicates, just put in variables
                let mut var = occurrences.pop().unwrap();
                var.is_duplicated = false;
                self.variables.insert(key, var);
            } else {
                // Sort by line number descending (highest first)
                occurrences.sort_by_key(|v| std::cmp::Reverse(v.line_number.unwrap_or(0)));

                // First one (highest line number) is the winner
                let mut winner = occurrences.remove(0);
                winner.is_duplicated = false;
                self.variables.insert(key, winner);

                // Rest are losers (marked as duplicated)
                for mut loser in occurrences {
                    loser.is_duplicated = true;
                    self.duplicated_vars.push(loser);
                }
            }
        }

        self.is_dirty = true;
    }

    /// Check if a key has duplicates (exists in duplicated_vars)
    pub fn has_duplicates(&self, key: &str) -> bool {
        self.duplicated_vars.iter().any(|v| v.key == key)
    }

    /// Get a mutable reference to a duplicated var by key and line number
    pub fn get_duplicated_var_mut(&mut self, key: &str, line_num: usize) -> Option<&mut EnvVar> {
        self.duplicated_vars.iter_mut().find(|v| v.key == key && v.line_number == Some(line_num))
    }
}
