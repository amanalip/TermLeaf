//! Editable startup settings read from the platform configuration directory.
//!
//! Phase 1 defines exactly one setting: `theme`. Precedence is built-in
//! default < `config.toml` < an explicit command-line option; resolution
//! happens in [`crate::process`] where the UI theme type is available. This
//! module stays independent of any UI crate, so it records the configured
//! slug verbatim and leaves validation to the resolver.
//!
//! A missing, unreadable, or malformed file falls back to defaults instead of
//! blocking startup; typed configuration errors arrive with the Phase 3
//! configuration cases.

use std::path::{Path, PathBuf};

use serde::Deserialize;

const PROJECT_QUALIFIER: &str = "org";
const PROJECT_ORGANIZATION: &str = "termleaf";
const PROJECT_APPLICATION: &str = "TermLeaf";
const CONFIG_DIRECTORY: &str = "termleaf";
const CONFIG_FILE: &str = "config.toml";

/// Settings recognized in `config.toml`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Settings {
    /// Configured theme slug such as `"paper"`, when one is present.
    pub theme: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    theme: Option<String>,
}

/// The platform configuration file path, when one can be resolved.
///
/// An explicit non-empty `XDG_CONFIG_HOME` wins on every platform so hermetic
/// test harnesses and container setups can relocate settings; otherwise the
/// native per-platform location from `directories` is used.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return Some(path_under(&home));
    }
    directories::ProjectDirs::from(PROJECT_QUALIFIER, PROJECT_ORGANIZATION, PROJECT_APPLICATION)
        .map(|dirs| dirs.config_dir().join(CONFIG_FILE))
}

/// The configuration file inside a configuration root directory.
///
/// Mirrors the relocation rule of [`default_path`] for tests and tooling.
#[must_use]
pub fn path_under(config_home: &Path) -> PathBuf {
    config_home.join(CONFIG_DIRECTORY).join(CONFIG_FILE)
}

impl Settings {
    /// Loads settings from the platform configuration location.
    #[must_use]
    pub fn load() -> Self {
        Self::load_from_opt(default_path().as_deref())
    }

    /// Loads settings from one explicit file path.
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        Self::load_from_opt(Some(path))
    }

    fn load_from_opt(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(file) = toml::from_str::<ConfigFile>(&contents) else {
            return Self::default();
        };
        Self { theme: file.theme }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(root: &Path, contents: &str) -> PathBuf {
        let path = root.join("config.toml");
        std::fs::write(&path, contents).expect("write config fixture");
        path
    }

    #[test]
    fn cfg_001_missing_or_unresolvable_config_loads_defaults() {
        let root = tempfile::tempdir().expect("config fixture root");
        let missing = root.path().join("absent").join("config.toml");

        assert_eq!(Settings::load_from(&missing), Settings::default());
        assert_eq!(Settings::load(), Settings::default());
    }

    #[test]
    fn cfg_002_every_theme_slug_round_trips_and_unknown_keys_are_ignored() {
        let root = tempfile::tempdir().expect("config fixture root");

        for slug in ["dark", "light", "high-contrast", "monochrome", "paper"] {
            let path = write_config(root.path(), &format!("theme = {slug:?}\n"));
            assert_eq!(
                Settings::load_from(&path).theme.as_deref(),
                Some(slug),
                "slug {slug} round trips"
            );
        }

        let extra = write_config(
            root.path(),
            "theme = \"light\"\n[future-section]\nkey = 7\n",
        );
        assert_eq!(
            Settings::load_from(&extra).theme.as_deref(),
            Some("light"),
            "unknown keys stay ignored"
        );
        let empty = write_config(root.path(), "");
        assert_eq!(Settings::load_from(&empty), Settings::default());
    }

    #[test]
    fn cfg_003_malformed_wrong_typed_and_unreadable_config_fall_back_to_defaults() {
        let root = tempfile::tempdir().expect("config fixture root");

        let syntax_error = write_config(root.path(), "theme = \n");
        assert_eq!(Settings::load_from(&syntax_error), Settings::default());

        let wrong_type = write_config(root.path(), "theme = 7\n");
        assert_eq!(Settings::load_from(&wrong_type), Settings::default());

        // A directory in place of the file cannot be read as TOML.
        let directory = root.path().join("directory");
        std::fs::create_dir(&directory).expect("create directory fixture");
        assert_eq!(Settings::load_from(&directory), Settings::default());
    }
}
