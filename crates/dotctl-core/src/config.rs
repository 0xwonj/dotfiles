use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use miette::{Context, IntoDiagnostic, Result, miette};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default)]
    pub github: bool,
    #[serde(default)]
    pub terminal_apps: bool,
    #[serde(default)]
    pub git_lfs: bool,
    #[serde(default)]
    pub ai_tools: bool,
    #[serde(default)]
    pub fastfetch: bool,
}

impl FeatureFlags {
    pub fn set_named(&mut self, key: &str, value: bool) -> Result<()> {
        match key {
            "github" => self.github = value,
            "terminal_apps" => self.terminal_apps = value,
            "git_lfs" => self.git_lfs = value,
            "ai_tools" => self.ai_tools = value,
            "fastfetch" => self.fastfetch = value,
            other => return Err(miette!("unknown feature key: {other}")),
        }
        Ok(())
    }

    pub fn enabled_features(&self) -> Vec<&'static str> {
        let mut enabled = Vec::new();
        if self.github {
            enabled.push("github");
        }
        if self.terminal_apps {
            enabled.push("terminal_apps");
        }
        if self.git_lfs {
            enabled.push("git_lfs");
        }
        if self.ai_tools {
            enabled.push("ai_tools");
        }
        if self.fastfetch {
            enabled.push("fastfetch");
        }
        enabled
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoConfig {
    #[serde(default)]
    pub source_dir: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitIdentity {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub signing_key: String,
    #[serde(default)]
    pub gpg_program: String,
    #[serde(default)]
    pub sign_commits: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentityConfig {
    #[serde(default)]
    pub git: GitIdentity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemConfig {
    #[serde(default)]
    pub package_manager_override: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    pub profile: String,
    pub repo: RepoConfig,
    #[serde(default)]
    pub features: FeatureFlags,
    #[serde(default)]
    pub identity: IdentityConfig,
    #[serde(default)]
    pub system: SystemConfig,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            profile: "default".to_string(),
            repo: RepoConfig::default(),
            features: FeatureFlags::default(),
            identity: IdentityConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default)]
    pub features: FeatureFlags,
    #[serde(default)]
    pub system: SystemConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundlePackages {
    #[serde(default)]
    pub required: Vec<BundlePackageGroup>,
    #[serde(default)]
    pub optional: Vec<BundlePackageGroup>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundlePackageGroup {
    pub manager: String,
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleTools {
    #[serde(default)]
    pub managed: Vec<String>,
    #[serde(default)]
    pub post_apply: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleSpec {
    pub id: String,
    #[serde(default)]
    pub feature: Option<String>,
    #[serde(default)]
    pub packages: BundlePackages,
    #[serde(default)]
    pub tools: BundleTools,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallerRegistry {
    #[serde(default)]
    pub installers: BTreeMap<String, InstallerSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallerSpec {
    pub kind: String,
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub verify_checksum: Option<bool>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub update_policy: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub install_dir: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub interpreter: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub asset_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub profile: String,
    pub repo: RepoConfig,
    pub features: FeatureFlags,
    pub identity: IdentityConfig,
    pub system: SystemConfig,
    pub platform: String,
    pub package_manager: String,
    pub applied_bundles: Vec<String>,
    pub last_applied_at: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedState {
    pub repo_root: PathBuf,
    pub profile_name: String,
    pub local: LocalConfig,
    pub package_manager: String,
    pub platform: String,
    pub applied_bundles: Vec<String>,
}

impl ResolvedState {
    pub fn feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "github" => self.local.features.github,
            "terminal_apps" => self.local.features.terminal_apps,
            "git_lfs" => self.local.features.git_lfs,
            "ai_tools" => self.local.features.ai_tools,
            "fastfetch" => self.local.features.fastfetch,
            _ => false,
        }
    }
}

pub fn load_profile(repo_root: &Path, name: &str) -> Result<ProfileConfig> {
    let path = repo_root
        .join("config/profiles")
        .join(format!("{name}.toml"));
    let raw = fs::read_to_string(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read profile {}", path.display()))?;
    toml::from_str(&raw)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to parse profile {}", path.display()))
}

pub fn load_bundle_specs(repo_root: &Path) -> Result<Vec<BundleSpec>> {
    let dir = repo_root.join("config/bundles");
    let mut specs = Vec::new();
    let mut entries = fs::read_dir(&dir)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read bundle directory {}", dir.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .into_diagnostic()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read bundle {}", path.display()))?;
        let spec: BundleSpec = toml::from_str(&raw)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to parse bundle {}", path.display()))?;
        specs.push(spec);
    }

    Ok(specs)
}

pub fn load_installer_registry(repo_root: &Path) -> Result<InstallerRegistry> {
    let path = repo_root.join("config/installers.toml");
    let raw = fs::read_to_string(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read installer registry {}", path.display()))?;
    toml::from_str(&raw)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to parse installer registry {}", path.display()))
}

pub fn load_local_config(path: &Path) -> Result<LocalConfig> {
    let raw = fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read local state {}", path.display()))?;
    toml::from_str(&raw)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to parse local state {}", path.display()))
}

pub fn write_local_config(path: &Path, config: &LocalConfig) -> Result<()> {
    let raw = toml::to_string_pretty(config).into_diagnostic()?;
    super::fsutil::write_secure_file(path, &raw)
}

pub fn write_snapshot(path: &Path, snapshot: &StateSnapshot) -> Result<()> {
    let raw = toml::to_string_pretty(snapshot).into_diagnostic()?;
    super::fsutil::write_secure_file(path, &raw)
}

pub fn available_profiles(repo_root: &Path) -> Result<Vec<String>> {
    let dir = repo_root.join("config/profiles");
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read profiles directory {}", dir.display()))?
    {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn repo_root_from_source_dir(source_dir: &str) -> Result<PathBuf> {
    let path = PathBuf::from(source_dir);
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .into_diagnostic()
            .map(|cwd| cwd.join(path))
    }
}
