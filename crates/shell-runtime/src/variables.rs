//! Variable store for the shell session.
//!
//! The store manages shell-local variables, exported variables, and the inherited
//! process environment. It provides a clean separation between variables that
//! exist only in the shell and those that should be passed to child processes.

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process;

/// Normalized key for environment variable lookups.
///
/// On Windows, environment variable names are case-insensitive. On Unix, they
/// are case-sensitive. This type normalizes keys appropriately for the platform.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NormalizedKey(String);

impl NormalizedKey {
    fn new(key: &str) -> Self {
        #[cfg(target_os = "windows")]
        let normalized = key.to_ascii_uppercase();
        #[cfg(not(target_os = "windows"))]
        let normalized = key.to_owned();
        Self(normalized)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Entry for a shell-local variable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VariableEntry {
    value: String,
    exported: bool,
}

/// Shell variable store.
///
/// Maintains three layers:
/// 1. Local variables (shell-only, not exported)
/// 2. Exported variables (passed to child processes)
/// 3. Inherited environment (from the parent process, minus hidden entries)
#[derive(Debug, Clone)]
pub struct VariableStore {
    /// Shell-local variables, including exported ones.
    locals: HashMap<NormalizedKey, VariableEntry>,
    /// Original spelling of local variable names. Windows lookups are
    /// case-insensitive, but display and restoration must preserve the case
    /// the user typed.
    local_names: HashMap<NormalizedKey, String>,
    /// Names of inherited environment variables that have been explicitly
    /// unset in this shell session.
    hidden_inherited: std::collections::HashSet<NormalizedKey>,
    /// Snapshot of the inherited environment at session creation.
    inherited: HashMap<NormalizedKey, String>,
    /// Original spelling of inherited environment variable names.
    inherited_names: HashMap<NormalizedKey, String>,
    /// Last command exit status ($?).
    last_status: i32,
    /// Shell process ID ($$).
    process_id: u32,
}

impl VariableStore {
    /// Creates a new variable store by snapshotting the current process environment.
    pub fn new() -> Self {
        let mut inherited = HashMap::new();
        let mut inherited_names = HashMap::new();
        for (key, value) in env::vars_os() {
            let key_str = key.to_string_lossy().into_owned();
            let value_str = value.to_string_lossy().into_owned();
            let normalized = NormalizedKey::new(&key_str);
            inherited.insert(normalized.clone(), value_str);
            inherited_names.insert(normalized, key_str);
        }

        // Initialize PWD from the current directory
        let pwd = env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::new());

        // Initialize HOME from environment or USERPROFILE on Windows
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| String::new());

        let mut store = Self {
            locals: HashMap::new(),
            local_names: HashMap::new(),
            hidden_inherited: std::collections::HashSet::new(),
            inherited,
            inherited_names,
            last_status: 0,
            process_id: process::id(),
        };

        // Initialize PWD and HOME as exported variables
        if !pwd.is_empty() {
            store.set_local("PWD", pwd, true);
        }
        if !home.is_empty() {
            store.set_local("HOME", home, true);
        }

        store
    }

    /// Creates a variable store with a custom environment for testing.
    pub fn with_environment(env: Vec<(String, String)>) -> Self {
        let mut inherited = HashMap::new();
        let mut inherited_names = HashMap::new();
        for (key, value) in env {
            let normalized = NormalizedKey::new(&key);
            inherited.insert(normalized.clone(), value);
            inherited_names.insert(normalized, key);
        }

        let mut store = Self {
            locals: HashMap::new(),
            local_names: HashMap::new(),
            hidden_inherited: std::collections::HashSet::new(),
            inherited,
            inherited_names,
            last_status: 0,
            process_id: process::id(),
        };

        // Initialize PWD and HOME if present in the provided environment
        if let Some(pwd) = store.inherited.get(&NormalizedKey::new("PWD")).cloned() {
            store.set_local("PWD", pwd, true);
        }
        if let Some(home) = store.inherited.get(&NormalizedKey::new("HOME")).cloned() {
            store.set_local("HOME", home, true);
        } else if let Some(userprofile) = store
            .inherited
            .get(&NormalizedKey::new("USERPROFILE"))
            .cloned()
        {
            store.set_local("HOME", userprofile, true);
        }

        store
    }

    /// Returns the value of a variable, checking locals first, then inherited.
    pub fn get(&self, name: &str) -> Option<String> {
        let key = NormalizedKey::new(name);

        // Check special variables first
        if name == "?" {
            return Some(self.last_status.to_string());
        }
        if name == "$" {
            return Some(self.process_id.to_string());
        }

        // Check locals (including exported)
        if let Some(entry) = self.locals.get(&key) {
            return Some(entry.value.clone());
        }

        // Check inherited (if not hidden)
        if !self.hidden_inherited.contains(&key) {
            return self.inherited.get(&key).cloned();
        }

        None
    }

    /// Returns the value of a local variable only (not inherited).
    pub fn get_local(&self, name: &str) -> Option<String> {
        let key = NormalizedKey::new(name);
        self.locals.get(&key).map(|e| e.value.clone())
    }

    /// Removes a local variable without hiding an inherited variable of the
    /// same name.
    ///
    /// This is used to restore function-parameter scope after a call. Unlike
    /// [`VariableStore::unset`], it does not shadow the inherited environment.
    pub fn remove_local(&mut self, name: &str) {
        let key = NormalizedKey::new(name);
        self.locals.remove(&key);
        self.local_names.remove(&key);
    }

    /// Sets a local variable.
    ///
    /// If `exported` is true, the variable will be passed to child processes.
    pub fn set_local(&mut self, name: &str, value: String, exported: bool) {
        if !shell_core::is_valid_variable_name(name) {
            return; // Silently ignore invalid names; caller should validate
        }

        let key = NormalizedKey::new(name);
        let keep_exported = exported
            || self.locals.get(&key).is_some_and(|entry| entry.exported)
            || (self.inherited.contains_key(&key) && !self.hidden_inherited.contains(&key));

        self.local_names.insert(key.clone(), name.to_owned());
        self.locals.insert(
            key,
            VariableEntry {
                value,
                exported: keep_exported,
            },
        );
    }

    /// Exports an existing local variable.
    pub fn export(&mut self, name: &str) -> bool {
        let key = NormalizedKey::new(name);
        if let Some(entry) = self.locals.get_mut(&key) {
            entry.exported = true;
            self.local_names.insert(key, name.to_owned());
            true
        } else {
            // Also check if it's in inherited environment
            if self.inherited.contains_key(&key) && !self.hidden_inherited.contains(&key) {
                // Re-export an inherited variable by copying it to locals
                if let Some(value) = self.inherited.get(&key).cloned() {
                    self.local_names.insert(key.clone(), name.to_owned());
                    self.locals.insert(
                        key,
                        VariableEntry {
                            value,
                            exported: true,
                        },
                    );
                    return true;
                }
            }
            false
        }
    }

    /// Sets a variable and exports it in one operation.
    pub fn set_exported(&mut self, name: &str, value: String) {
        if shell_core::is_valid_variable_name(name) {
            let key = NormalizedKey::new(name);
            self.local_names.insert(key.clone(), name.to_owned());
            self.locals.insert(
                key,
                VariableEntry {
                    value,
                    exported: true,
                },
            );
        }
    }

    /// Unsets a variable (removes from locals and hides from inherited).
    ///
    /// Returns `true` on success (always succeeds, even for nonexistent variables,
    /// matching POSIX unset behavior).
    pub fn unset(&mut self, name: &str) -> bool {
        let key = NormalizedKey::new(name);
        self.locals.remove(&key);
        self.local_names.remove(&key);
        self.hidden_inherited.insert(key);
        true
    }

    /// Returns all exported variables (locals + inherited, respecting hidden).
    pub fn exported_env(&self) -> Vec<(String, String)> {
        let mut env = Vec::new();

        // Add exported locals
        for (key, entry) in &self.locals {
            if entry.exported {
                let name = self
                    .local_names
                    .get(key)
                    .map_or_else(|| key.as_str().to_owned(), Clone::clone);
                env.push((name, entry.value.clone()));
            }
        }

        // Add inherited variables that are not hidden and not overridden by locals
        for (key, value) in &self.inherited {
            if !self.hidden_inherited.contains(key) && !self.locals.contains_key(key) {
                let name = self
                    .inherited_names
                    .get(key)
                    .map_or_else(|| key.as_str().to_owned(), Clone::clone);
                env.push((name, value.clone()));
            }
        }

        env
    }

    /// Returns the effective environment for child processes.
    ///
    /// This includes exported locals and non-hidden inherited variables.
    pub fn child_environment(
        &self,
        overrides: &[(OsString, OsString)],
    ) -> Vec<(OsString, OsString)> {
        let mut env: Vec<(OsString, OsString)> = self
            .exported_env()
            .into_iter()
            .map(|(k, v)| (OsString::from(k), OsString::from(v)))
            .collect();

        // Apply overrides (they take precedence)
        for (k, v) in overrides {
            // Remove any existing entry with the same key (case-insensitive on Windows)
            let norm_k = NormalizedKey::new(&k.to_string_lossy());
            env.retain(|(existing_k, _)| {
                NormalizedKey::new(&existing_k.to_string_lossy()) != norm_k
            });
            env.push((k.clone(), v.clone()));
        }

        env
    }

    /// Returns all local variables (exported and non-exported).
    pub fn local_variables(&self) -> Vec<(String, String, bool)> {
        let mut vars: Vec<_> = self
            .locals
            .iter()
            .map(|(k, e)| {
                let name = self
                    .local_names
                    .get(k)
                    .map_or_else(|| k.as_str().to_owned(), Clone::clone);
                (name, e.value.clone(), e.exported)
            })
            .collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        vars
    }

    /// Returns all inherited environment variables (excluding hidden).
    pub fn inherited_variables(&self) -> Vec<(String, String)> {
        let mut vars: Vec<_> = self
            .inherited
            .iter()
            .filter(|(k, _)| !self.hidden_inherited.contains(k))
            .map(|(k, v)| {
                let name = self
                    .inherited_names
                    .get(k)
                    .map_or_else(|| k.as_str().to_owned(), Clone::clone);
                (name, v.clone())
            })
            .collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        vars
    }

    /// Updates the PWD variable.
    pub fn set_pwd(&mut self, path: &Path) {
        let value = path.to_string_lossy().into_owned();
        self.set_local("PWD", value, true);
    }

    /// Gets the current PWD value.
    pub fn get_pwd(&self) -> Option<String> {
        self.get("PWD")
    }

    /// Sets the last command exit status ($?).
    pub fn set_last_status(&mut self, status: i32) {
        self.last_status = status;
    }

    /// Gets the last command exit status.
    pub fn last_status(&self) -> i32 {
        self.last_status
    }

    /// Gets the shell process ID ($$).
    pub fn process_id(&self) -> u32 {
        self.process_id
    }
}

impl Default for VariableStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn new_store_snapshots_environment() {
        let store = VariableStore::new();
        // Should have at least some environment variables
        assert!(!store.inherited.is_empty());
    }

    #[test]
    fn local_variable_creation_and_lookup() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "test_value".to_owned(), false);
        assert_eq!(store.get("TEST_VAR"), Some("test_value".to_owned()));
        assert_eq!(store.get_local("TEST_VAR"), Some("test_value".to_owned()));
    }

    #[test]
    fn local_variable_is_not_exported_by_default() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "test_value".to_owned(), false);
        let exported = store.exported_env();
        assert!(!exported.iter().any(|(k, _)| k == "TEST_VAR"));
    }

    #[test]
    fn exported_variable_appears_in_child_environment() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "test_value".to_owned(), true);
        let exported = store.exported_env();
        assert!(
            exported
                .iter()
                .any(|(k, v)| k == "TEST_VAR" && v == "test_value")
        );
    }

    #[test]
    fn export_marks_existing_local_as_exported() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "test_value".to_owned(), false);
        assert!(store.export("TEST_VAR"));
        let exported = store.exported_env();
        assert!(
            exported
                .iter()
                .any(|(k, v)| k == "TEST_VAR" && v == "test_value")
        );
    }

    #[test]
    fn export_inherited_variable_copies_to_locals() {
        let mut store = VariableStore::with_environment(vec![(
            "INHERITED_VAR".to_owned(),
            "from_env".to_owned(),
        )]);
        assert!(store.export("INHERITED_VAR"));
        let exported = store.exported_env();
        assert!(
            exported
                .iter()
                .any(|(k, v)| k == "INHERITED_VAR" && v == "from_env")
        );
    }

    #[test]
    fn reassigning_an_exported_local_preserves_export_status() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "first".to_owned(), true);
        store.set_local("TEST_VAR", "second".to_owned(), false);

        let exported = store.exported_env();
        assert!(
            exported
                .iter()
                .any(|(name, value)| name == "TEST_VAR" && value == "second"),
            "{exported:?}"
        );
    }

    #[test]
    fn reassigning_an_inherited_variable_keeps_it_exported() {
        let mut store = VariableStore::with_environment(vec![(
            "INHERITED_VAR".to_owned(),
            "from_env".to_owned(),
        )]);
        store.set_local("INHERITED_VAR", "overridden".to_owned(), false);

        let exported = store.exported_env();
        assert_eq!(
            exported
                .iter()
                .filter(|(name, _)| name == "INHERITED_VAR")
                .map(|(_, value)| value.as_str())
                .collect::<Vec<_>>(),
            ["overridden"]
        );
    }

    #[test]
    fn unset_then_assign_does_not_reexport_an_inherited_variable() {
        let mut store = VariableStore::with_environment(vec![(
            "INHERITED_VAR".to_owned(),
            "from_env".to_owned(),
        )]);
        assert!(store.unset("INHERITED_VAR"));
        store.set_local("INHERITED_VAR", "local".to_owned(), false);

        assert!(
            !store
                .exported_env()
                .iter()
                .any(|(name, _)| name == "INHERITED_VAR")
        );
    }

    #[test]
    fn unset_removes_local_variable() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("TEST_VAR", "test_value".to_owned(), false);
        assert!(store.unset("TEST_VAR"));
        assert_eq!(store.get("TEST_VAR"), None);
    }

    #[test]
    fn unset_hides_inherited_variable() {
        let mut store = VariableStore::with_environment(vec![(
            "INHERITED_VAR".to_owned(),
            "from_env".to_owned(),
        )]);
        assert!(store.unset("INHERITED_VAR"));
        assert_eq!(store.get("INHERITED_VAR"), None);
        // But it should still be in inherited, just hidden
        assert!(
            store
                .inherited_variables()
                .iter()
                .all(|(k, _)| k != "INHERITED_VAR")
        );
    }

    #[test]
    fn unset_nonexistent_succeeds() {
        let mut store = VariableStore::with_environment(vec![]);
        assert!(store.unset("NONEXISTENT"));
    }

    #[test]
    fn child_environment_includes_exported_locals_and_inherited() {
        let mut store =
            VariableStore::with_environment(vec![("INHERITED".to_owned(), "from_env".to_owned())]);
        store.set_local("LOCAL_EXPORTED", "local_val".to_owned(), true);
        store.set_local("LOCAL_ONLY", "local_only".to_owned(), false);

        let child_env = store.child_environment(&[]);
        let keys: Vec<_> = child_env
            .iter()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        assert!(keys.contains(&"INHERITED".to_owned()));
        assert!(keys.contains(&"LOCAL_EXPORTED".to_owned()));
        assert!(!keys.contains(&"LOCAL_ONLY".to_owned()));
    }

    #[test]
    fn child_environment_overrides_take_precedence() {
        let mut store =
            VariableStore::with_environment(vec![("VAR".to_owned(), "original".to_owned())]);
        store.set_local("VAR", "local".to_owned(), true);

        let overrides = vec![(OsString::from("VAR"), OsString::from("override"))];
        let child_env = store.child_environment(&overrides);

        let var_value = child_env
            .iter()
            .find(|(k, _)| k == "VAR")
            .map(|(_, v)| v.to_string_lossy().into_owned());
        assert_eq!(var_value, Some("override".to_owned()));
    }

    #[test]
    fn special_variable_last_status() {
        let mut store = VariableStore::with_environment(vec![]);
        assert_eq!(store.get("?"), Some("0".to_owned()));
        store.set_last_status(42);
        assert_eq!(store.get("?"), Some("42".to_owned()));
    }

    #[test]
    fn special_variable_process_id() {
        let store = VariableStore::with_environment(vec![]);
        assert_eq!(store.get("$"), Some(store.process_id.to_string()));
    }

    #[test]
    fn set_pwd_updates_pwd_variable() {
        let mut store = VariableStore::with_environment(vec![]);
        let path = PathBuf::from("/new/path");
        store.set_pwd(&path);
        assert_eq!(store.get("PWD"), Some("/new/path".to_owned()));
        // PWD should be exported
        let exported = store.exported_env();
        assert!(exported.iter().any(|(k, v)| k == "PWD" && v == "/new/path"));
    }

    #[test]
    fn local_variable_list_preserves_the_original_spelling() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("MixedCase", "value".to_owned(), false);

        let vars = store.local_variables();
        assert!(
            vars.iter()
                .any(|(name, value, _)| name == "MixedCase" && value == "value")
        );
    }

    #[test]
    fn local_variables_list_includes_export_flag() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("EXPORTED", "val1".to_owned(), true);
        store.set_local("LOCAL", "val2".to_owned(), false);

        let vars = store.local_variables();
        assert_eq!(vars.len(), 2);
        let exported = vars.iter().find(|(k, _, _)| k == "EXPORTED").unwrap();
        assert_eq!(exported.1, "val1");
        assert!(exported.2);
        let local = vars.iter().find(|(k, _, _)| k == "LOCAL").unwrap();
        assert_eq!(local.1, "val2");
        assert!(!local.2);
    }

    #[test]
    fn inherited_variables_list_excludes_hidden() {
        let mut store = VariableStore::with_environment(vec![
            ("VISIBLE".to_owned(), "val1".to_owned()),
            ("HIDDEN".to_owned(), "val2".to_owned()),
        ]);
        store.unset("HIDDEN");

        let vars = store.inherited_variables();
        assert!(vars.iter().any(|(k, v)| k == "VISIBLE" && v == "val1"));
        assert!(!vars.iter().any(|(k, _)| k == "HIDDEN"));
    }

    #[test]
    fn invalid_variable_names_are_rejected() {
        let mut store = VariableStore::with_environment(vec![]);
        store.set_local("123INVALID", "value".to_owned(), false);
        assert_eq!(store.get("123INVALID"), None);
    }
}
