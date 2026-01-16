use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::{Environment, EnvironmentType};

/// Represents a project (directory containing .env files)
#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub environments: BTreeMap<EnvironmentType, Environment>,
    all_keys_cache: Vec<String>,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            environments: BTreeMap::new(),
            all_keys_cache: Vec::new(),
        }
    }

    pub fn add_environment(&mut self, env: Environment) {
        self.environments.insert(env.env_type.clone(), env);
        self.rebuild_keys_cache();
    }

    /// Get all unique keys across all environments
    pub fn all_keys(&self) -> &[String] {
        &self.all_keys_cache
    }

    /// Rebuild the keys cache from all environments
    pub fn rebuild_keys_cache(&mut self) {
        let mut keys: BTreeSet<String> = BTreeSet::new();
        for env in self.environments.values() {
            for key in env.variables.keys() {
                keys.insert(key.clone());
            }
        }
        self.all_keys_cache = keys.into_iter().collect();
    }

    /// Get value for a key in a specific environment
    pub fn get_value(&self, key: &str, env_type: &EnvironmentType) -> Option<&str> {
        self.environments
            .get(env_type)
            .and_then(|env| env.get_value(key))
    }

    /// Set value for a key in the default environment
    pub fn set_default_value(&mut self, key: String, value: String) {
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.set(key, value);
        }
    }

    /// Copy a value from one environment to the default environment
    pub fn copy_to_default(&mut self, key: &str, from_env: &EnvironmentType) {
        if let Some(value) = self.get_value(key, from_env).map(String::from) {
            self.set_default_value(key.to_string(), value);
        }
    }

    /// Bulk switch: replace entire .env with values from another environment
    /// Creates .env if it doesn't exist (useful for copying from K8s configs)
    pub fn bulk_switch(&mut self, from_env: &EnvironmentType) {
        if let Some(source) = self.environments.get(from_env).cloned() {
            // Create Default environment if it doesn't exist
            if !self.environments.contains_key(&EnvironmentType::Default) {
                let env_path = self.path.join(".env");
                let new_env = Environment::new(EnvironmentType::Default, env_path);
                self.environments.insert(EnvironmentType::Default, new_env);
            }

            if let Some(default) = self.environments.get_mut(&EnvironmentType::Default) {
                default.variables.clear();
                for (key, mut var) in source.variables {
                    // Reset line numbers for newly copied vars
                    var.line_number = None;
                    var.section = None;
                    default.variables.insert(key, var);
                }
                default.is_dirty = true;
            }
        }
    }

    /// Check if project has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.environments.values().any(|env| env.is_dirty)
    }

    /// Get sorted environment types (Default first, then alphabetical)
    pub fn sorted_env_types(&self) -> Vec<&EnvironmentType> {
        let mut types: Vec<_> = self.environments.keys().collect();
        types.sort_by_key(|t| t.sort_key());
        types
    }

    /// Add a new key to the default environment
    pub fn add_key(&mut self, key: String, value: String) {
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.set(key.clone(), value);
        }
        self.rebuild_keys_cache();
    }

    /// Delete a key from all environments
    pub fn delete_key(&mut self, key: &str) {
        for env in self.environments.values_mut() {
            env.remove(key);
        }
        self.rebuild_keys_cache();
    }

    /// Delete a key from a specific environment
    pub fn delete_key_from_env(&mut self, key: &str, env_type: &EnvironmentType) {
        if let Some(env) = self.environments.get_mut(env_type) {
            env.remove(key);
        }
        self.rebuild_keys_cache();
    }

    /// Get keys that exist in .env (default environment)
    pub fn default_keys(&self) -> Vec<&String> {
        self.environments
            .get(&EnvironmentType::Default)
            .map(|env| env.variables.keys().collect())
            .unwrap_or_default()
    }

    /// Get keys that exist in a specific env but NOT in .env
    pub fn missing_keys_for_env(&self, env_type: &EnvironmentType) -> Vec<&String> {
        let default_keys: BTreeSet<_> = self
            .environments
            .get(&EnvironmentType::Default)
            .map(|env| env.variables.keys().collect())
            .unwrap_or_default();

        self.environments
            .get(env_type)
            .map(|env| {
                env.variables
                    .keys()
                    .filter(|k| !default_keys.contains(k))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find which environment the current .env value matches, or None if custom
    pub fn find_matching_env(&self, key: &str) -> Option<EnvironmentType> {
        let default_value = self.get_value(key, &EnvironmentType::Default)?;

        // Check each named environment to see if value matches
        for (env_type, env) in &self.environments {
            if *env_type == EnvironmentType::Default {
                continue;
            }
            if let Some(env_value) = env.get_value(key) {
                if env_value == default_value {
                    return Some(env_type.clone());
                }
            }
        }

        None // Custom value - doesn't match any env
    }

    /// Find ALL environments where the current .env value matches
    /// Returns empty Vec if custom value (doesn't match any env)
    pub fn find_all_matching_envs(&self, key: &str) -> Vec<EnvironmentType> {
        let Some(default_value) = self.get_value(key, &EnvironmentType::Default) else {
            return Vec::new();
        };

        let mut matching = Vec::new();
        for (env_type, env) in &self.environments {
            if *env_type == EnvironmentType::Default {
                continue;
            }
            if let Some(env_value) = env.get_value(key) {
                if env_value == default_value {
                    matching.push(env_type.clone());
                }
            }
        }

        matching
    }

    /// Get non-default environment types only
    pub fn named_env_types(&self) -> Vec<&EnvironmentType> {
        self.environments
            .keys()
            .filter(|t| **t != EnvironmentType::Default)
            .collect()
    }

    /// Set value in .env from a specific environment
    pub fn set_value_from_env(&mut self, key: &str, from_env: &EnvironmentType) {
        if let Some(value) = self.get_value(key, from_env).map(String::from) {
            self.set_default_value(key.to_string(), value);
        }
    }

    /// Get a summary of which environments have matching values in current config
    /// Returns (list of env names with matches, has_custom_values)
    pub fn env_value_summary(&self) -> (Vec<String>, bool) {
        let mut matching_envs: BTreeSet<String> = BTreeSet::new();
        let mut has_custom = false;

        let default_keys = self.default_keys();
        for key in default_keys {
            match self.find_matching_env(key) {
                Some(env_type) => {
                    matching_envs.insert(env_type.display_name().to_string());
                }
                None => {
                    has_custom = true;
                }
            }
        }

        (matching_envs.into_iter().collect(), has_custom)
    }

    /// Get all unique sections from the default environment, in order of first appearance
    pub fn all_sections(&self) -> Vec<Option<String>> {
        let mut sections: Vec<Option<String>> = Vec::new();
        let mut seen: BTreeSet<Option<String>> = BTreeSet::new();

        if let Some(env) = self.environments.get(&EnvironmentType::Default) {
            // Collect all variables (including duplicated) with line numbers
            let mut all_vars: Vec<(usize, Option<String>)> = Vec::new();

            // Add regular variables
            for var in env.variables.values() {
                all_vars.push((var.line_number.unwrap_or(usize::MAX), var.section.clone()));
            }

            // Add duplicated variables
            for var in &env.duplicated_vars {
                all_vars.push((var.line_number.unwrap_or(usize::MAX), var.section.clone()));
            }

            // Sort by line number
            all_vars.sort_by_key(|(line, _)| *line);

            for (_, section) in all_vars {
                if !seen.contains(&section) {
                    seen.insert(section.clone());
                    sections.push(section);
                }
            }
        }

        sections
    }

    /// Get keys in the default environment that belong to a specific section
    pub fn keys_for_section(&self, section: &Option<String>) -> Vec<&String> {
        self.environments
            .get(&EnvironmentType::Default)
            .map(|env| {
                let mut keys: Vec<_> = env
                    .variables
                    .iter()
                    .filter(|(_, var)| var.section == *section)
                    .collect();
                // Sort by line number
                keys.sort_by_key(|(_, var)| var.line_number.unwrap_or(usize::MAX));
                keys.into_iter().map(|(k, _)| k).collect()
            })
            .unwrap_or_default()
    }

    /// Get environments that have a specific section defined
    pub fn envs_with_section(&self, section: &str) -> Vec<&EnvironmentType> {
        self.environments
            .iter()
            .filter(|(env_type, env)| {
                **env_type != EnvironmentType::Default
                    && env.variables.values().any(|v| {
                        v.section.as_ref().map(|s| s == section).unwrap_or(false)
                    })
            })
            .map(|(env_type, _)| env_type)
            .collect()
    }

    /// Find matching environments for all keys in a section
    /// Returns the intersection of matching envs across all keys in the section
    /// Returns empty Vec if keys have different matching envs (custom section)
    pub fn find_section_matching_envs(&self, section: &str) -> Vec<EnvironmentType> {
        let section_keys = self.keys_for_section(&Some(section.to_string()));
        if section_keys.is_empty() {
            return Vec::new();
        }

        // Get matching envs for the first key
        let mut intersection: BTreeSet<EnvironmentType> = self
            .find_all_matching_envs(section_keys[0])
            .into_iter()
            .collect();

        // Intersect with matching envs of all other keys
        for key in section_keys.iter().skip(1) {
            let key_envs: BTreeSet<EnvironmentType> = self
                .find_all_matching_envs(key)
                .into_iter()
                .collect();
            intersection = intersection.intersection(&key_envs).cloned().collect();
        }

        intersection.into_iter().collect()
    }

    /// Switch all keys in a section to values from another environment
    /// This removes all existing keys in the section and replaces them with keys from the source env
    pub fn switch_section(&mut self, section: &str, from_env: &EnvironmentType) {
        // First, remove all keys in this section from default
        if let Some(default_env) = self.environments.get_mut(&EnvironmentType::Default) {
            let keys_to_remove: Vec<String> = default_env
                .variables
                .iter()
                .filter(|(_, var)| var.section.as_ref().map(|s| s == section).unwrap_or(false))
                .map(|(k, _)| k.clone())
                .collect();

            for key in keys_to_remove {
                default_env.variables.remove(&key);
            }
            default_env.is_dirty = true;
        }

        // Get keys for the section from the source environment
        let keys_to_add: Vec<(String, super::EnvVar)> = self
            .environments
            .get(from_env)
            .map(|env| {
                env.variables
                    .iter()
                    .filter(|(_, var)| {
                        var.section.as_ref().map(|s| s == section).unwrap_or(false)
                    })
                    .map(|(k, v)| {
                        let mut new_var = super::EnvVar::new(k.clone(), v.value.clone());
                        new_var = new_var.with_section(section.to_string());
                        if let Some(ln) = v.line_number {
                            new_var = new_var.with_line_number(ln);
                        }
                        (k.clone(), new_var)
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Add keys from source env to default
        if let Some(default_env) = self.environments.get_mut(&EnvironmentType::Default) {
            for (key, var) in keys_to_add {
                default_env.variables.insert(key, var);
            }
            default_env.is_dirty = true;
        }

        // Rebuild the keys cache
        self.rebuild_keys_cache();
    }

    /// Get all displayable items (sections, keys, and duplicated keys) for the default environment
    /// Returns items grouped by section:
    /// - Each section name appears once with all its keys grouped under it
    /// - Sections are ordered by the minimum line number of their keys
    /// - Keys within each section are ordered by line number
    pub fn display_items(&self) -> Vec<super::DisplayItem> {
        let Some(env) = self.environments.get(&EnvironmentType::Default) else {
            return Vec::new();
        };

        // Group all items by section
        // section -> Vec<(line_number, DisplayItem)>
        let mut section_items: std::collections::BTreeMap<Option<String>, Vec<(usize, super::DisplayItem)>> =
            std::collections::BTreeMap::new();

        // Add regular keys
        for (key, var) in &env.variables {
            let line = var.line_number.unwrap_or(usize::MAX);
            section_items
                .entry(var.section.clone())
                .or_default()
                .push((line, super::DisplayItem::Key(key.clone())));
        }

        // Add duplicated keys
        for dup_var in &env.duplicated_vars {
            let line = dup_var.line_number.unwrap_or(usize::MAX);
            section_items
                .entry(dup_var.section.clone())
                .or_default()
                .push((
                    line,
                    super::DisplayItem::DuplicatedKey(
                        dup_var.key.clone(),
                        dup_var.value.clone(),
                        line,
                    ),
                ));
        }

        // Sort keys within each section by line number
        for items in section_items.values_mut() {
            items.sort_by_key(|(line, _)| *line);
        }

        // Calculate min line number for each section (used for section ordering)
        let mut section_order: Vec<(usize, Option<String>)> = section_items
            .iter()
            .map(|(section, items)| {
                let min_line = items.iter().map(|(line, _)| *line).min().unwrap_or(usize::MAX);
                (min_line, section.clone())
            })
            .collect();

        // Sort sections by their minimum line number
        section_order.sort_by_key(|(min_line, _)| *min_line);

        // Build final display list
        let mut items = Vec::new();
        for (_, section) in section_order {
            // Add section header (if named section)
            if let Some(ref section_name) = section {
                items.push(super::DisplayItem::Section(section_name.clone()));
            }

            // Add all keys in this section
            if let Some(section_keys) = section_items.get(&section) {
                for (_, item) in section_keys {
                    items.push(item.clone());
                }
            }
        }

        items
    }

    /// Get the number of displayable items (sections + keys)
    pub fn display_item_count(&self) -> usize {
        self.display_items().len()
    }

    /// Move an item up
    /// - For keys in a section: only moves within the same section
    /// - For top-level keys (no section): can swap with adjacent sections
    /// - For section headers: moves entire section up (swaps with section/keys above)
    /// Returns the new display index after the move, or None if move not possible
    pub fn move_item_up(&mut self, display_index: usize) -> Option<usize> {
        let items = self.display_items();
        if display_index == 0 || display_index >= items.len() {
            return None;
        }

        let item = items[display_index].clone();

        // Handle section header movement
        if let super::DisplayItem::Section(section_name) = &item {
            return self.move_section_up(section_name);
        }

        // For keys, get the section it belongs to
        let item_section = self.get_item_section(&item);

        // Find the nearest key above
        let above_idx = display_index - 1;

        // Check what's above us
        match &items[above_idx] {
            super::DisplayItem::Section(section_name) => {
                // Hit a section header
                if item_section.is_none() {
                    // Top-level key can swap with this section
                    return self.move_toplevel_key_above_section(&item, section_name);
                }
                // Key in a named section hit its own header, can't move up further
                return None;
            }
            super::DisplayItem::Key(_) | super::DisplayItem::DuplicatedKey(_, _, _) => {
                // Found a key - check if it's in the same section
                let above_section = self.get_item_section(&items[above_idx]);
                if above_section != item_section {
                    // Different section
                    if item_section.is_none() {
                        // Top-level key can swap with the section above
                        if let Some(ref section_name) = above_section {
                            return self.move_toplevel_key_above_section(&item, section_name);
                        }
                    }
                    // Key in a named section can't cross boundary
                    return None;
                }
                // Same section, we can swap with this key
            }
        }

        let above_item = items[above_idx].clone();

        // Get the key name for tracking
        let key = match &item {
            super::DisplayItem::Key(k) => k.clone(),
            super::DisplayItem::DuplicatedKey(k, _, _) => k.clone(),
            _ => return None,
        };
        let target_line = self.get_item_line(&above_item)?;

        // Swap line numbers (within same section)
        self.swap_item_lines_only(&item, &above_item)?;

        // Recalculate duplicates (may change winner/loser)
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.recalculate_duplicates();
        }

        // Find where our item ended up
        self.find_item_by_key_and_line(&key, target_line)
    }

    /// Move an item down
    /// - For keys in a section: only moves within the same section
    /// - For top-level keys (no section): can swap with adjacent sections
    /// - For section headers: moves entire section down (swaps with section/keys below)
    /// Returns the new display index after the move, or None if move not possible
    pub fn move_item_down(&mut self, display_index: usize) -> Option<usize> {
        let items = self.display_items();
        if display_index >= items.len().saturating_sub(1) {
            return None;
        }

        let item = items[display_index].clone();

        // Handle section header movement
        if let super::DisplayItem::Section(section_name) = &item {
            return self.move_section_down(section_name);
        }

        // For keys, get the section it belongs to
        let item_section = self.get_item_section(&item);

        // Find the nearest item below
        let below_idx = display_index + 1;

        // Check what's below us
        match &items[below_idx] {
            super::DisplayItem::Section(section_name) => {
                // Hit a section header
                if item_section.is_none() {
                    // Top-level key can swap with this section
                    return self.move_toplevel_key_below_section(&item, section_name);
                }
                // Key in a named section hit another section's header, can't move down
                return None;
            }
            super::DisplayItem::Key(_) | super::DisplayItem::DuplicatedKey(_, _, _) => {
                // Found a key - check if it's in the same section
                let below_section = self.get_item_section(&items[below_idx]);
                if below_section != item_section {
                    // Different section
                    if item_section.is_none() {
                        // Top-level key can swap with the section below
                        if let Some(ref section_name) = below_section {
                            return self.move_toplevel_key_below_section(&item, section_name);
                        }
                    }
                    // Key in a named section can't cross boundary
                    return None;
                }
                // Same section, we can swap with this key
            }
        }

        let below_item = items[below_idx].clone();

        // Get the key name for tracking
        let key = match &item {
            super::DisplayItem::Key(k) => k.clone(),
            super::DisplayItem::DuplicatedKey(k, _, _) => k.clone(),
            _ => return None,
        };
        let target_line = self.get_item_line(&below_item)?;

        // Swap line numbers (within same section)
        self.swap_item_lines_only(&item, &below_item)?;

        // Recalculate duplicates (may change winner/loser)
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.recalculate_duplicates();
        }

        // Find where our item ended up
        self.find_item_by_key_and_line(&key, target_line)
    }

    /// Move an entire section up (swap with the section or top-level keys above)
    /// Returns the new display index of the section header, or None if move not possible
    fn move_section_up(&mut self, section_name: &str) -> Option<usize> {
        // Get ordered list of sections (by min line number)
        let section_order = self.get_section_order();

        // Find our section's position in the order
        let our_pos = section_order.iter().position(|s| s.as_ref() == Some(&section_name.to_string()))?;

        // Can't move up if already first
        if our_pos == 0 {
            return None;
        }

        // Get the section above us
        let section_above = &section_order[our_pos - 1];

        // Swap the line numbers of all keys in both sections
        self.swap_section_lines(&Some(section_name.to_string()), section_above)?;

        // Recalculate duplicates
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.recalculate_duplicates();
        }

        // Find the new display index of our section header
        let items = self.display_items();
        items.iter().position(|item| {
            matches!(item, super::DisplayItem::Section(name) if name == section_name)
        })
    }

    /// Move an entire section down (swap with the section or top-level keys below)
    /// Returns the new display index of the section header, or None if move not possible
    fn move_section_down(&mut self, section_name: &str) -> Option<usize> {
        // Get ordered list of sections (by min line number)
        let section_order = self.get_section_order();

        // Find our section's position in the order
        let our_pos = section_order.iter().position(|s| s.as_ref() == Some(&section_name.to_string()))?;

        // Can't move down if already last
        if our_pos >= section_order.len() - 1 {
            return None;
        }

        // Get the section below us
        let section_below = &section_order[our_pos + 1];

        // Swap the line numbers of all keys in both sections
        self.swap_section_lines(&Some(section_name.to_string()), section_below)?;

        // Recalculate duplicates
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            env.recalculate_duplicates();
        }

        // Find the new display index of our section header
        let items = self.display_items();
        items.iter().position(|item| {
            matches!(item, super::DisplayItem::Section(name) if name == section_name)
        })
    }

    /// Move a top-level key above a named section
    /// The key gets a line number smaller than the section's minimum line
    fn move_toplevel_key_above_section(&mut self, item: &super::DisplayItem, section_name: &str) -> Option<usize> {
        let (key, old_line, is_dup) = match item {
            super::DisplayItem::Key(k) => {
                let line = self.get_item_line(item)?;
                (k.clone(), line, false)
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => (k.clone(), *ln, true),
            _ => return None,
        };

        // Find the minimum line number in the target section
        let section_min = self.min_line_in_section(&Some(section_name.to_string()));

        // New line number is just before the section
        let new_line = if section_min > 0 { section_min - 1 } else { 0 };

        // Update the key's line number
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            if is_dup {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == key && v.line_number == Some(old_line)) {
                    var.line_number = Some(new_line);
                }
            } else if let Some(var) = env.variables.get_mut(&key) {
                var.line_number = Some(new_line);
            }
            env.is_dirty = true;
            env.recalculate_duplicates();
        }

        // Find where the key ended up
        self.find_item_by_key_and_line(&key, new_line)
    }

    /// Move a top-level key below a named section
    /// The key gets a line number larger than the section's maximum line
    fn move_toplevel_key_below_section(&mut self, item: &super::DisplayItem, section_name: &str) -> Option<usize> {
        let (key, old_line, is_dup) = match item {
            super::DisplayItem::Key(k) => {
                let line = self.get_item_line(item)?;
                (k.clone(), line, false)
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => (k.clone(), *ln, true),
            _ => return None,
        };

        // Find the maximum line number in the target section
        let section_max = self.max_line_in_section(&Some(section_name.to_string()));

        // New line number is just after the section
        let new_line = section_max + 1;

        // Update the key's line number
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            if is_dup {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == key && v.line_number == Some(old_line)) {
                    var.line_number = Some(new_line);
                }
            } else if let Some(var) = env.variables.get_mut(&key) {
                var.line_number = Some(new_line);
            }
            env.is_dirty = true;
            env.recalculate_duplicates();
        }

        // Find where the key ended up
        self.find_item_by_key_and_line(&key, new_line)
    }

    /// Get the order of sections (None = top-level keys, Some(name) = named section)
    /// Ordered by minimum line number of keys in each section
    fn get_section_order(&self) -> Vec<Option<String>> {
        let Some(env) = self.environments.get(&EnvironmentType::Default) else {
            return Vec::new();
        };

        // Group keys by section and get min line for each
        let mut section_min_lines: std::collections::BTreeMap<Option<String>, usize> =
            std::collections::BTreeMap::new();

        for var in env.variables.values() {
            let line = var.line_number.unwrap_or(usize::MAX);
            section_min_lines
                .entry(var.section.clone())
                .and_modify(|min| *min = (*min).min(line))
                .or_insert(line);
        }

        for var in &env.duplicated_vars {
            let line = var.line_number.unwrap_or(usize::MAX);
            section_min_lines
                .entry(var.section.clone())
                .and_modify(|min| *min = (*min).min(line))
                .or_insert(line);
        }

        // Sort sections by their min line number
        let mut sections: Vec<_> = section_min_lines.into_iter().collect();
        sections.sort_by_key(|(_, min_line)| *min_line);

        sections.into_iter().map(|(section, _)| section).collect()
    }

    /// Swap the line number ranges of two sections
    /// After swap, section1 will have the line numbers that section2 had, and vice versa
    fn swap_section_lines(&mut self, section1: &Option<String>, section2: &Option<String>) -> Option<()> {
        let env = self.environments.get_mut(&EnvironmentType::Default)?;

        // Get all line numbers for each section (from both variables and duplicated_vars)
        let mut lines1: Vec<usize> = Vec::new();
        let mut lines2: Vec<usize> = Vec::new();

        for var in env.variables.values() {
            if var.section == *section1 {
                if let Some(line) = var.line_number {
                    lines1.push(line);
                }
            } else if var.section == *section2 {
                if let Some(line) = var.line_number {
                    lines2.push(line);
                }
            }
        }

        for var in &env.duplicated_vars {
            if var.section == *section1 {
                if let Some(line) = var.line_number {
                    lines1.push(line);
                }
            } else if var.section == *section2 {
                if let Some(line) = var.line_number {
                    lines2.push(line);
                }
            }
        }

        if lines1.is_empty() || lines2.is_empty() {
            return None;
        }

        // Sort both lists
        lines1.sort();
        lines2.sort();

        // Determine which section is "above" (smaller line numbers)
        let (upper_section, upper_lines, lower_section, lower_lines) =
            if lines1[0] < lines2[0] {
                (section1, lines1, section2, lines2)
            } else {
                (section2, lines2, section1, lines1)
            };

        // Calculate new line numbers:
        // Upper section gets line numbers starting after where lower section ends
        // Lower section gets line numbers starting at where upper section started
        let upper_start = upper_lines[0];
        let lower_count = lower_lines.len();

        // New assignments:
        // - Lower section items get lines starting at upper_start
        // - Upper section items get lines starting at upper_start + lower_count

        // Create mapping: old_line -> new_line
        let mut line_mapping: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();

        // Lower section items move to the top
        for (i, &old_line) in lower_lines.iter().enumerate() {
            line_mapping.insert(old_line, upper_start + i);
        }

        // Upper section items move below
        for (i, &old_line) in upper_lines.iter().enumerate() {
            line_mapping.insert(old_line, upper_start + lower_count + i);
        }

        // Apply the mapping
        for var in env.variables.values_mut() {
            if var.section == *lower_section || var.section == *upper_section {
                if let Some(old_line) = var.line_number {
                    if let Some(&new_line) = line_mapping.get(&old_line) {
                        var.line_number = Some(new_line);
                    }
                }
            }
        }

        for var in env.duplicated_vars.iter_mut() {
            if var.section == *lower_section || var.section == *upper_section {
                if let Some(old_line) = var.line_number {
                    if let Some(&new_line) = line_mapping.get(&old_line) {
                        var.line_number = Some(new_line);
                    }
                }
            }
        }

        env.is_dirty = true;
        Some(())
    }

    /// Move a key into the section above it
    /// Returns the new display index, or None if not possible
    pub fn move_key_into_section_above(&mut self, display_index: usize) -> Option<usize> {
        let items = self.display_items();
        if display_index == 0 || display_index >= items.len() {
            return None;
        }

        let item = &items[display_index];

        // Must be a key (not a section)
        let (key, old_line, is_dup) = match item {
            super::DisplayItem::Key(k) => {
                let line = self.get_item_line(item)?;
                (k.clone(), line, false)
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => (k.clone(), *ln, true),
            super::DisplayItem::Section(_) => return None,
        };

        // Must not already be in a section
        if self.get_item_section(item).is_some() {
            return None;
        }

        // Find the section above
        let section_above = self.find_section_above(display_index)?;
        let max_line = self.max_line_in_section(&Some(section_above.clone()));

        // Update the item
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            if is_dup {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == key && v.line_number == Some(old_line)) {
                    var.section = Some(section_above);
                    var.line_number = Some(max_line + 1);
                }
            } else if let Some(var) = env.variables.get_mut(&key) {
                var.section = Some(section_above);
                var.line_number = Some(max_line + 1);
            }
            env.recalculate_duplicates();
        }

        Some(display_index)
    }

    /// Move a key out of its section to top-level
    /// Returns the new display index, or None if not possible
    pub fn move_key_out_of_section(&mut self, display_index: usize) -> Option<usize> {
        let items = self.display_items();
        if display_index >= items.len() {
            return None;
        }

        let item = &items[display_index];

        // Must be a key (not a section)
        let (key, old_line, is_dup) = match item {
            super::DisplayItem::Key(k) => {
                let line = self.get_item_line(item)?;
                (k.clone(), line, false)
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => (k.clone(), *ln, true),
            super::DisplayItem::Section(_) => return None,
        };

        // Must be in a section
        if self.get_item_section(item).is_none() {
            return None;
        }

        // Get new line number (before all sections)
        let min_line = self.min_line_in_section(&None);
        let new_line = if min_line > 0 { min_line - 1 } else { 0 };

        // Update the item
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            if is_dup {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == key && v.line_number == Some(old_line)) {
                    var.section = None;
                    var.line_number = Some(new_line);
                }
            } else if let Some(var) = env.variables.get_mut(&key) {
                var.section = None;
                var.line_number = Some(new_line);
            }
            env.recalculate_duplicates();
        }

        self.renumber_lines();
        Some(0)
    }

    /// Get the section for any display item
    fn get_item_section(&self, item: &super::DisplayItem) -> Option<String> {
        let env = self.environments.get(&EnvironmentType::Default)?;
        match item {
            super::DisplayItem::Key(key) => {
                env.variables.get(key).and_then(|v| v.section.clone())
            }
            super::DisplayItem::DuplicatedKey(key, _, line_num) => {
                env.duplicated_vars
                    .iter()
                    .find(|v| v.key == *key && v.line_number == Some(*line_num))
                    .and_then(|v| v.section.clone())
            }
            super::DisplayItem::Section(_) => None,
        }
    }

    /// Get line number for any display item
    fn get_item_line(&self, item: &super::DisplayItem) -> Option<usize> {
        let env = self.environments.get(&EnvironmentType::Default)?;
        match item {
            super::DisplayItem::Key(key) => {
                env.variables.get(key).and_then(|v| v.line_number)
            }
            super::DisplayItem::DuplicatedKey(_, _, line_num) => Some(*line_num),
            super::DisplayItem::Section(_) => None,
        }
    }

    /// Find the section name above a given display index
    fn find_section_above(&self, display_index: usize) -> Option<String> {
        let items = self.display_items();

        // Look backwards from current position for a section header
        for i in (0..display_index).rev() {
            if let super::DisplayItem::Section(name) = &items[i] {
                return Some(name.clone());
            }
        }
        None
    }

    /// Get the maximum line number in a section
    fn max_line_in_section(&self, section: &Option<String>) -> usize {
        self.environments
            .get(&EnvironmentType::Default)
            .map(|env| {
                env.variables
                    .values()
                    .filter(|v| v.section == *section)
                    .filter_map(|v| v.line_number)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    /// Get the minimum line number in a section
    fn min_line_in_section(&self, section: &Option<String>) -> usize {
        self.environments
            .get(&EnvironmentType::Default)
            .map(|env| {
                env.variables
                    .values()
                    .filter(|v| v.section == *section)
                    .filter_map(|v| v.line_number)
                    .min()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    /// Renumber all lines sequentially to maintain proper order
    fn renumber_lines(&mut self) {
        if let Some(env) = self.environments.get_mut(&EnvironmentType::Default) {
            // Collect all vars sorted by current line number and section
            let mut vars: Vec<_> = env.variables.values().cloned().collect();
            vars.sort_by_key(|v| (v.section.clone(), v.line_number.unwrap_or(usize::MAX)));

            // Renumber
            for (i, var) in vars.iter().enumerate() {
                if let Some(v) = env.variables.get_mut(&var.key) {
                    v.line_number = Some(i + 1);
                }
            }
        }
    }

    /// Swap line numbers between two display items (line numbers only, NOT section membership)
    fn swap_item_lines_only(&mut self, item1: &super::DisplayItem, item2: &super::DisplayItem) -> Option<()> {
        let env = self.environments.get_mut(&EnvironmentType::Default)?;

        // Get line numbers for both items
        let line1 = match item1 {
            super::DisplayItem::Key(k) => env.variables.get(k)?.line_number?,
            super::DisplayItem::DuplicatedKey(_, _, ln) => *ln,
            super::DisplayItem::Section(_) => return None,
        };

        let line2 = match item2 {
            super::DisplayItem::Key(k) => env.variables.get(k)?.line_number?,
            super::DisplayItem::DuplicatedKey(_, _, ln) => *ln,
            super::DisplayItem::Section(_) => return None,
        };

        // Update item1's line number only
        match item1 {
            super::DisplayItem::Key(k) => {
                if let Some(var) = env.variables.get_mut(k) {
                    var.line_number = Some(line2);
                }
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == *k && v.line_number == Some(*ln)) {
                    var.line_number = Some(line2);
                }
            }
            super::DisplayItem::Section(_) => {}
        }

        // Update item2's line number only
        match item2 {
            super::DisplayItem::Key(k) => {
                if let Some(var) = env.variables.get_mut(k) {
                    var.line_number = Some(line1);
                }
            }
            super::DisplayItem::DuplicatedKey(k, _, ln) => {
                if let Some(var) = env.duplicated_vars.iter_mut()
                    .find(|v| v.key == *k && v.line_number == Some(*ln)) {
                    var.line_number = Some(line1);
                }
            }
            super::DisplayItem::Section(_) => {}
        }

        env.is_dirty = true;
        Some(())
    }

    /// Find the display index of an item by its key and the line number it now occupies
    fn find_item_by_key_and_line(&self, key: &str, target_line: usize) -> Option<usize> {
        let items = self.display_items();

        // Search for the item with this key at this line
        for (idx, item) in items.iter().enumerate() {
            match item {
                super::DisplayItem::Key(k) if k == key => {
                    // Check if this key has the target line
                    if let Some(env) = self.environments.get(&EnvironmentType::Default) {
                        if let Some(var) = env.variables.get(k) {
                            if var.line_number == Some(target_line) {
                                return Some(idx);
                            }
                        }
                    }
                }
                super::DisplayItem::DuplicatedKey(k, _, ln) if k == key && *ln == target_line => {
                    return Some(idx);
                }
                _ => {}
            }
        }

        // Fallback: just find any occurrence of the key
        for (idx, item) in items.iter().enumerate() {
            match item {
                super::DisplayItem::Key(k) if k == key => return Some(idx),
                super::DisplayItem::DuplicatedKey(k, _, _) if k == key => return Some(idx),
                _ => {}
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvVar, Environment};
    use std::path::PathBuf;

    fn create_test_project() -> Project {
        let mut project = Project::new("test".to_string(), PathBuf::from("/tmp/test"));
        let mut env = Environment::new(EnvironmentType::Default, PathBuf::from("/tmp/test/.env"));

        // Add keys with line numbers
        let var1 = EnvVar::new("KEY_A".to_string(), "value_a".to_string())
            .with_line_number(1);
        let var2 = EnvVar::new("KEY_B".to_string(), "value_b".to_string())
            .with_line_number(2);
        let var3 = EnvVar::new("KEY_C".to_string(), "value_c".to_string())
            .with_line_number(3);

        env.variables.insert("KEY_A".to_string(), var1);
        env.variables.insert("KEY_B".to_string(), var2);
        env.variables.insert("KEY_C".to_string(), var3);

        project.add_environment(env);
        project
    }

    fn create_test_project_with_duplicates() -> Project {
        let mut project = Project::new("test".to_string(), PathBuf::from("/tmp/test"));
        let mut env = Environment::new(EnvironmentType::Default, PathBuf::from("/tmp/test/.env"));

        // Add regular keys
        let var1 = EnvVar::new("KEY_A".to_string(), "value_a".to_string())
            .with_line_number(1);
        let var_dup_winner = EnvVar::new("KEY_DUP".to_string(), "winner_value".to_string())
            .with_line_number(4);
        let var3 = EnvVar::new("KEY_C".to_string(), "value_c".to_string())
            .with_line_number(5);

        env.variables.insert("KEY_A".to_string(), var1);
        env.variables.insert("KEY_DUP".to_string(), var_dup_winner);
        env.variables.insert("KEY_C".to_string(), var3);

        // Add duplicated key (earlier occurrence)
        let mut var_dup_loser = EnvVar::new("KEY_DUP".to_string(), "loser_value".to_string())
            .with_line_number(2);
        var_dup_loser.is_duplicated = true;
        env.duplicated_vars.push(var_dup_loser);

        project.add_environment(env);
        project
    }

    #[test]
    fn test_display_items_order() {
        let project = create_test_project();
        let items = project.display_items();

        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "KEY_A"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "KEY_B"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "KEY_C"));
    }

    #[test]
    fn test_display_items_with_duplicates() {
        let project = create_test_project_with_duplicates();
        let items = project.display_items();

        // Should have: KEY_A (line 1), KEY_DUP loser (line 2), KEY_DUP winner (line 4), KEY_C (line 5)
        assert_eq!(items.len(), 4);
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "KEY_A"));
        assert!(matches!(&items[1], super::super::DisplayItem::DuplicatedKey(k, _, _) if k == "KEY_DUP"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "KEY_DUP"));
        assert!(matches!(&items[3], super::super::DisplayItem::Key(k) if k == "KEY_C"));
    }

    #[test]
    fn test_move_item_up() {
        let mut project = create_test_project();

        // Move KEY_B (index 1) up
        let new_idx = project.move_item_up(1);
        assert!(new_idx.is_some());

        // After move, KEY_B should be at index 0
        let items = project.display_items();
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "KEY_B"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "KEY_A"));
    }

    #[test]
    fn test_move_item_down() {
        let mut project = create_test_project();

        // Move KEY_A (index 0) down
        let new_idx = project.move_item_down(0);
        assert!(new_idx.is_some());

        // After move, KEY_A should be at index 1
        let items = project.display_items();
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "KEY_B"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "KEY_A"));
    }

    #[test]
    fn test_move_at_boundary() {
        let mut project = create_test_project();

        // Can't move first item up
        assert!(project.move_item_up(0).is_none());

        // Can't move last item down
        assert!(project.move_item_down(2).is_none());
    }

    #[test]
    fn test_move_duplicated_key_down_becomes_winner() {
        let mut project = create_test_project_with_duplicates();

        // Initially: KEY_A(1), KEY_DUP loser(2), KEY_DUP winner(4), KEY_C(5)
        // Move the loser (index 1) down past the winner

        // First move: loser is now at line 4, winner at line 2
        let items_before = project.display_items();
        assert_eq!(items_before.len(), 4);

        // Move index 1 down
        let _ = project.move_item_down(1);

        // After one move, the loser should now be between the positions
        // The loser had line 2, winner had line 4
        // After swap: loser has line 4, winner has line 2
        // recalculate_duplicates: highest line wins, so loser becomes winner

        let items = project.display_items();
        // The former "loser" is now winner (highest line = 4)
        // The former "winner" is now loser (line = 2)

        // Order should be: KEY_A(1), KEY_DUP loser(2), KEY_DUP winner(4), KEY_C(5)
        // But loser and winner switched roles!
        assert_eq!(items.len(), 4);
    }

    fn create_test_project_with_sections() -> Project {
        let mut project = Project::new("test".to_string(), PathBuf::from("/tmp/test"));
        let mut env = Environment::new(EnvironmentType::Default, PathBuf::from("/tmp/test/.env"));

        // Section 1: Database
        let mut var1 = EnvVar::new("DB_HOST".to_string(), "localhost".to_string())
            .with_line_number(2);
        var1.section = Some("Database".to_string());
        let mut var2 = EnvVar::new("DB_PORT".to_string(), "5432".to_string())
            .with_line_number(3);
        var2.section = Some("Database".to_string());

        // Section 2: Cache
        let mut var3 = EnvVar::new("REDIS_HOST".to_string(), "localhost".to_string())
            .with_line_number(5);
        var3.section = Some("Cache".to_string());
        let mut var4 = EnvVar::new("REDIS_PORT".to_string(), "6379".to_string())
            .with_line_number(6);
        var4.section = Some("Cache".to_string());

        env.variables.insert("DB_HOST".to_string(), var1);
        env.variables.insert("DB_PORT".to_string(), var2);
        env.variables.insert("REDIS_HOST".to_string(), var3);
        env.variables.insert("REDIS_PORT".to_string(), var4);

        project.add_environment(env);
        project
    }

    #[test]
    fn test_keys_cannot_move_across_sections() {
        let mut project = create_test_project_with_sections();

        // Display: [Database], DB_HOST, DB_PORT, [Cache], REDIS_HOST, REDIS_PORT
        let items = project.display_items();
        assert_eq!(items.len(), 6);
        assert!(matches!(&items[0], super::super::DisplayItem::Section(s) if s == "Database"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "DB_HOST"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "DB_PORT"));
        assert!(matches!(&items[3], super::super::DisplayItem::Section(s) if s == "Cache"));

        // Try to move DB_PORT (index 2) down - should fail because next item is section header
        let result = project.move_item_down(2);
        assert!(result.is_none(), "Should NOT be able to move key across section boundary");

        // Try to move REDIS_HOST (index 4) up - should fail because previous item is section header
        let result = project.move_item_up(4);
        assert!(result.is_none(), "Should NOT be able to move key across section boundary");

        // Verify items haven't changed
        let items = project.display_items();
        assert_eq!(items.len(), 6);
    }

    #[test]
    fn test_move_within_section() {
        let mut project = create_test_project_with_sections();

        // Move DB_PORT (index 2) up within Database section
        let new_idx = project.move_item_up(2);
        assert!(new_idx.is_some(), "Should be able to move within section");

        let items = project.display_items();
        // After move: [Database] (0), DB_PORT (1), DB_HOST (2), [Cache] (3)...
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "DB_PORT"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "DB_HOST"));
    }

    #[test]
    fn test_move_duplicate_returns_correct_index() {
        let mut project = create_test_project_with_duplicates();

        // Initially: KEY_A(1:idx0), KEY_DUP loser(2:idx1), KEY_DUP winner(4:idx2), KEY_C(5:idx3)
        let items_before = project.display_items();
        assert_eq!(items_before.len(), 4);

        // Move the loser (index 1) down - it should swap with winner (index 2)
        let new_idx = project.move_item_down(1);

        // After the move, our item (now at line 4, becoming winner) should be at index 2
        assert_eq!(new_idx, Some(2), "Index should point to the moved item's new position");

        // Verify the item at index 2 has value "loser_value" (since that's what we moved)
        // After recalculation, it's now the winner (Key), but has the same value
        let items = project.display_items();
        if let super::super::DisplayItem::Key(k) = &items[2] {
            // The key at index 2 should be KEY_DUP (the one we moved, now winner)
            assert_eq!(k, "KEY_DUP");
        } else {
            panic!("Expected Key at index 2");
        }
    }

    fn create_test_project_with_toplevel_and_sections() -> Project {
        let mut project = Project::new("test".to_string(), PathBuf::from("/tmp/test"));
        let mut env = Environment::new(EnvironmentType::Default, PathBuf::from("/tmp/test/.env"));

        // Top-level key (no section)
        let var_top = EnvVar::new("TOP_KEY".to_string(), "top_value".to_string())
            .with_line_number(1);

        // Section: Database
        let mut var_db1 = EnvVar::new("DB_HOST".to_string(), "localhost".to_string())
            .with_line_number(3);
        var_db1.section = Some("Database".to_string());
        let mut var_db2 = EnvVar::new("DB_PORT".to_string(), "5432".to_string())
            .with_line_number(4);
        var_db2.section = Some("Database".to_string());

        env.variables.insert("TOP_KEY".to_string(), var_top);
        env.variables.insert("DB_HOST".to_string(), var_db1);
        env.variables.insert("DB_PORT".to_string(), var_db2);

        project.add_environment(env);
        project
    }

    #[test]
    fn test_toplevel_key_can_move_across_sections() {
        let mut project = create_test_project_with_toplevel_and_sections();

        // Initial display:
        // TOP_KEY (line 1, section=None)
        // [Database] header
        // DB_HOST (line 3, section=Database)
        // DB_PORT (line 4, section=Database)
        let items = project.display_items();
        assert_eq!(items.len(), 4);
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "TOP_KEY"));
        assert!(matches!(&items[1], super::super::DisplayItem::Section(s) if s == "Database"));

        // Move TOP_KEY (index 0) down - should succeed, moves below the Database section
        let result = project.move_item_down(0);
        assert!(result.is_some(), "Top-level key should be able to move past sections");

        // After move, TOP_KEY should be after the Database section
        // Display: [Database], DB_HOST, DB_PORT, TOP_KEY
        let items = project.display_items();
        assert_eq!(items.len(), 4);
        assert!(matches!(&items[0], super::super::DisplayItem::Section(s) if s == "Database"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "DB_HOST"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "DB_PORT"));
        assert!(matches!(&items[3], super::super::DisplayItem::Key(k) if k == "TOP_KEY"));

        // Now move TOP_KEY back up
        let result = project.move_item_up(3);
        assert!(result.is_some(), "Top-level key should be able to move back up past sections");

        // After move, TOP_KEY should be before the Database section again
        let items = project.display_items();
        assert!(matches!(&items[0], super::super::DisplayItem::Key(k) if k == "TOP_KEY"));
    }

    #[test]
    fn test_move_section_up() {
        let mut project = create_test_project_with_sections();

        // Display: [Database](0), DB_HOST(1), DB_PORT(2), [Cache](3), REDIS_HOST(4), REDIS_PORT(5)
        let items = project.display_items();
        assert_eq!(items.len(), 6);
        assert!(matches!(&items[0], super::super::DisplayItem::Section(s) if s == "Database"));
        assert!(matches!(&items[3], super::super::DisplayItem::Section(s) if s == "Cache"));

        // Move Cache section (index 3) up - should swap with Database section
        let new_idx = project.move_item_up(3);
        assert!(new_idx.is_some(), "Should be able to move section up");

        // After move: [Cache], REDIS_HOST, REDIS_PORT, [Database], DB_HOST, DB_PORT
        let items = project.display_items();
        assert_eq!(items.len(), 6);
        assert!(matches!(&items[0], super::super::DisplayItem::Section(s) if s == "Cache"));
        assert!(matches!(&items[1], super::super::DisplayItem::Key(k) if k == "REDIS_HOST"));
        assert!(matches!(&items[2], super::super::DisplayItem::Key(k) if k == "REDIS_PORT"));
        assert!(matches!(&items[3], super::super::DisplayItem::Section(s) if s == "Database"));
        assert!(matches!(&items[4], super::super::DisplayItem::Key(k) if k == "DB_HOST"));
        assert!(matches!(&items[5], super::super::DisplayItem::Key(k) if k == "DB_PORT"));
    }

    #[test]
    fn test_move_section_down() {
        let mut project = create_test_project_with_sections();

        // Display: [Database](0), DB_HOST(1), DB_PORT(2), [Cache](3), REDIS_HOST(4), REDIS_PORT(5)
        let items = project.display_items();
        assert_eq!(items.len(), 6);

        // Move Database section (index 0) down - should swap with Cache section
        let new_idx = project.move_item_down(0);
        assert!(new_idx.is_some(), "Should be able to move section down");

        // After move: [Cache], REDIS_HOST, REDIS_PORT, [Database], DB_HOST, DB_PORT
        let items = project.display_items();
        assert_eq!(items.len(), 6);
        assert!(matches!(&items[0], super::super::DisplayItem::Section(s) if s == "Cache"));
        assert!(matches!(&items[3], super::super::DisplayItem::Section(s) if s == "Database"));
    }

    #[test]
    fn test_section_movement_preserves_key_order() {
        let mut project = create_test_project_with_sections();

        // Move Cache section up
        let _ = project.move_item_up(3);

        // Keys within each section should maintain their relative order
        let items = project.display_items();

        // Find Cache section keys
        let cache_keys: Vec<_> = items.iter().filter_map(|item| {
            if let super::super::DisplayItem::Key(k) = item {
                if k.starts_with("REDIS") {
                    return Some(k.clone());
                }
            }
            None
        }).collect();

        // REDIS_HOST should come before REDIS_PORT (original order preserved)
        let host_pos = cache_keys.iter().position(|k| k == "REDIS_HOST").unwrap();
        let port_pos = cache_keys.iter().position(|k| k == "REDIS_PORT").unwrap();
        assert!(host_pos < port_pos, "Key order within section should be preserved");

        // Same for Database section
        let db_keys: Vec<_> = items.iter().filter_map(|item| {
            if let super::super::DisplayItem::Key(k) = item {
                if k.starts_with("DB") {
                    return Some(k.clone());
                }
            }
            None
        }).collect();

        let host_pos = db_keys.iter().position(|k| k == "DB_HOST").unwrap();
        let port_pos = db_keys.iter().position(|k| k == "DB_PORT").unwrap();
        assert!(host_pos < port_pos, "Key order within section should be preserved");
    }

    #[test]
    fn test_bulk_switch_creates_env_from_k8s() {
        // Create a project with only a K8s environment (no .env)
        let mut project = Project::new("test".to_string(), PathBuf::from("/tmp/test"));

        let k8s_env_type = EnvironmentType::Kubernetes {
            resource_name: "my-app".to_string(),
            container_name: None,
        };
        let mut k8s_env = Environment::new(k8s_env_type.clone(), PathBuf::from("/tmp/test/.k8s/deployment.yaml"));

        let var1 = EnvVar::new("SECRET".to_string(), "abc123".to_string());
        let var2 = EnvVar::new("PORT".to_string(), "8080".to_string());
        k8s_env.variables.insert("SECRET".to_string(), var1);
        k8s_env.variables.insert("PORT".to_string(), var2);

        project.add_environment(k8s_env);

        // Verify no Default environment exists
        assert!(!project.environments.contains_key(&EnvironmentType::Default));

        // Bulk switch from K8s to Default - should create Default
        project.bulk_switch(&k8s_env_type);

        // Verify Default environment was created
        assert!(project.environments.contains_key(&EnvironmentType::Default));

        // Verify values were copied
        let default_env = project.environments.get(&EnvironmentType::Default).unwrap();
        assert_eq!(default_env.get_value("SECRET"), Some("abc123"));
        assert_eq!(default_env.get_value("PORT"), Some("8080"));
        assert!(default_env.is_dirty);

        // Verify the path is correct
        assert_eq!(default_env.path, PathBuf::from("/tmp/test/.env"));
    }
}
