use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Local;
use dialoguer::{Confirm, Input, Select};
use miette::{Context, IntoDiagnostic, Result, miette};
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::ui::Ui;

use crate::config::{
    BundleSpec, FeatureFlags, IdentityConfig, InstallerRegistry, InstallerSpec, LocalConfig,
    ProfileConfig, RepoConfig, ResolvedState, StateSnapshot, available_profiles, load_bundle_specs,
    load_installer_registry, load_local_config, load_profile, write_local_config, write_snapshot,
};
use crate::fsutil::{expand_tilde, write_secure_file};
use crate::system::{
    GitHubAsset, GitHubRelease, PackageManagerId, Runtime, detect_package_manager,
    package_available, package_command_available, package_installed,
    pacman_selective_upgrade_supported, platform_name,
};

#[derive(Debug, Clone, Default)]
pub struct BootstrapOptions {
    pub repo: Option<PathBuf>,
    pub profile: Option<String>,
    pub package_manager: Option<String>,
    pub with_github: bool,
    pub with_terminal_apps: bool,
    pub with_git_lfs: bool,
    pub with_ai_tools: bool,
    pub with_fastfetch: bool,
    pub git_name: Option<String>,
    pub git_email: Option<String>,
    pub git_signing_key: Option<String>,
    pub git_gpg_program: Option<String>,
    pub sign_commits: bool,
    pub no_check: bool,
    pub no_prompt: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    pub no_check: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {}

#[derive(Debug, Clone, Default)]
pub struct DiffOptions {}

#[derive(Debug, Clone, Default)]
pub struct DoctorOptions {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOutcome {
    Clean,
    Dirty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorOutcome {
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateShowTarget {
    Local,
    Snapshot,
}

#[derive(Debug, Serialize)]
struct ChezmoiConfig<'a> {
    #[serde(rename = "sourceDir")]
    source_dir: &'a str,
    data: ChezmoiData<'a>,
}

#[derive(Debug, Serialize)]
struct ChezmoiData<'a> {
    repo: &'a RepoConfig,
    features: &'a FeatureFlags,
    identity: &'a IdentityConfig,
}

pub struct App {
    runtime: Runtime,
    ui: Ui,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            runtime: Runtime::detect()?,
            ui: Ui::detect(),
        })
    }

    pub fn from_runtime(runtime: Runtime) -> Self {
        Self {
            runtime,
            ui: Ui::detect(),
        }
    }

    pub fn bootstrap(&self, options: BootstrapOptions) -> Result<()> {
        let repo_root = canonical_repo_root(options.repo.clone())?;
        let mut local = if self.local_config_path().is_file() {
            load_local_config(&self.local_config_path())?
        } else {
            self.initialize_local_config(&repo_root, &options)?
        };

        if let Some(profile) = &options.profile {
            local.profile = profile.clone();
        }
        if let Some(name) = &options.git_name {
            local.identity.git.name = name.clone();
        }
        if let Some(email) = &options.git_email {
            local.identity.git.email = email.clone();
        }
        if let Some(signing_key) = &options.git_signing_key {
            local.identity.git.signing_key = signing_key.clone();
        }
        if let Some(gpg_program) = &options.git_gpg_program {
            local.identity.git.gpg_program = gpg_program.clone();
        }
        if options.sign_commits {
            local.identity.git.sign_commits = true;
        }
        if let Some(pm) = &options.package_manager {
            local.system.package_manager_override = pm.clone();
        }
        if options.with_github {
            local.features.github = true;
        }
        if options.with_terminal_apps {
            local.features.terminal_apps = true;
        }
        if options.with_git_lfs {
            local.features.git_lfs = true;
        }
        if options.with_ai_tools {
            local.features.ai_tools = true;
        }
        if options.with_fastfetch {
            local.features.fastfetch = true;
        }
        local.repo.source_dir = repo_root.to_string_lossy().into_owned();

        self.validate_local(&local, options.no_prompt)?;
        write_local_config(&self.local_config_path(), &local)?;

        let resolved = self.resolve_state(&repo_root, local)?;
        self.execute(&resolved, true, !options.no_check)?;
        Ok(())
    }

    pub fn update(&self, options: UpdateOptions) -> Result<()> {
        let local = self.require_local_config()?;
        let repo_root = canonical_repo_root(Some(PathBuf::from(&local.repo.source_dir)))?;
        let resolved = self.resolve_state(&repo_root, local)?;
        self.execute(&resolved, true, !options.no_check)
    }

    pub fn apply(&self, _options: ApplyOptions) -> Result<()> {
        let local = self.require_local_config()?;
        let repo_root = canonical_repo_root(Some(PathBuf::from(&local.repo.source_dir)))?;
        let resolved = self.resolve_state(&repo_root, local)?;
        self.generate_chezmoi_config(&resolved)?;
        self.ensure_local_extension_points()?;
        self.apply_chezmoi()?;
        self.run_minimal_post_apply_tasks(&resolved)?;
        Ok(())
    }

    pub fn diff(&self, _options: DiffOptions) -> Result<DiffOutcome> {
        let local = self.require_local_config()?;
        let repo_root = canonical_repo_root(Some(PathBuf::from(&local.repo.source_dir)))?;
        let resolved = self.resolve_state(&repo_root, local)?;
        self.generate_chezmoi_config(&resolved)?;
        self.ensure_local_extension_points()?;
        let status = self
            .runtime
            .command("chezmoi")
            .arg("diff")
            .status()
            .into_diagnostic()
            .wrap_err("failed to run chezmoi diff")?;
        match status.code() {
            Some(0) => {
                self.summary("No managed file changes", true);
                Ok(DiffOutcome::Clean)
            }
            Some(1) => {
                self.summary("Managed file changes detected", false);
                Ok(DiffOutcome::Dirty)
            }
            _ => Err(miette!("chezmoi diff failed")),
        }
    }

    pub fn doctor(&self, _options: DoctorOptions) -> Result<DoctorOutcome> {
        let local = self.require_local_config()?;
        let repo_root = canonical_repo_root(Some(PathBuf::from(&local.repo.source_dir)))?;
        let resolved = self.resolve_state(&repo_root, local)?;
        let ok = self.run_doctor(&resolved)?;
        self.summary("Doctor completed", ok);
        Ok(if ok {
            DoctorOutcome::Healthy
        } else {
            DoctorOutcome::Unhealthy
        })
    }

    pub fn show_state(&self, target: StateShowTarget) -> Result<String> {
        let path = match target {
            StateShowTarget::Local => self.local_config_path(),
            StateShowTarget::Snapshot => self.snapshot_path(),
        };
        let raw = fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read {}", path.display()))?;
        Ok(raw)
    }

    pub fn edit_state(&self) -> Result<()> {
        let path = self.local_config_path();
        if !path.is_file() {
            return Err(miette!(
                "missing local state: {}. Run 'dotctl bootstrap' first",
                path.display()
            ));
        }
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let status = Command::new("sh")
            .arg("-lc")
            .arg(format!(
                "{} {}",
                shell_escape(&editor),
                shell_escape(path.to_string_lossy().as_ref())
            ))
            .status()
            .into_diagnostic()
            .wrap_err("failed to launch editor")?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("editor exited unsuccessfully"))
        }
    }

    pub fn features_list(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "github",
                "Install GitHub CLI and enable the GitHub feature bundle.",
            ),
            (
                "terminal_apps",
                "Install tmux/btop and user-local starship/yazi plus terminal dotfiles.",
            ),
            (
                "git_lfs",
                "Install git-lfs and render LFS filters into ~/.gitconfig.",
            ),
            ("ai_tools", "Install Codex CLI and Claude Code."),
            (
                "fastfetch",
                "Manage fastfetch config under ~/.config/fastfetch.",
            ),
        ]
    }

    fn execute(&self, resolved: &ResolvedState, upgrade: bool, run_check: bool) -> Result<()> {
        self.section("resolve state");
        self.detail("repo", &resolved.repo_root.display().to_string());
        self.detail("profile", &resolved.profile_name);
        self.detail("package mgr", &resolved.package_manager);

        self.install_packages(resolved, upgrade)?;
        self.install_managed_tools(resolved, upgrade)?;
        self.generate_chezmoi_config(resolved)?;
        self.ensure_local_extension_points()?;
        self.apply_chezmoi()?;
        self.run_post_apply_tasks(resolved, upgrade)?;
        if run_check {
            let healthy = self.run_doctor(resolved)?;
            if !healthy {
                return Err(miette!("doctor checks failed"));
            }
        }
        self.write_snapshot(resolved)?;
        Ok(())
    }

    fn initialize_local_config(
        &self,
        repo_root: &Path,
        options: &BootstrapOptions,
    ) -> Result<LocalConfig> {
        let profiles = available_profiles(repo_root)?;
        let selected_profile = if let Some(profile) = &options.profile {
            profile.clone()
        } else if options.no_prompt {
            "default".to_string()
        } else {
            let index = profiles
                .iter()
                .position(|name| name == "default")
                .unwrap_or(0);
            let chosen = Select::new()
                .with_prompt("Select workstation profile")
                .items(&profiles)
                .default(index)
                .interact()
                .into_diagnostic()?;
            profiles[chosen].clone()
        };
        let profile = load_profile(repo_root, &selected_profile)?;
        let mut local = LocalConfig {
            profile: selected_profile,
            repo: RepoConfig {
                source_dir: repo_root.to_string_lossy().into_owned(),
            },
            features: profile.features.clone(),
            identity: IdentityConfig::default(),
            system: profile.system.clone(),
        };

        local.features.github |= options.with_github;
        local.features.terminal_apps |= options.with_terminal_apps;
        local.features.git_lfs |= options.with_git_lfs;
        local.features.ai_tools |= options.with_ai_tools;
        local.features.fastfetch |= options.with_fastfetch;
        if let Some(package_manager) = &options.package_manager {
            local.system.package_manager_override = package_manager.clone();
        }
        if let Some(name) = &options.git_name {
            local.identity.git.name = name.clone();
        }
        if let Some(email) = &options.git_email {
            local.identity.git.email = email.clone();
        }
        if let Some(signing_key) = &options.git_signing_key {
            local.identity.git.signing_key = signing_key.clone();
        }
        if let Some(gpg_program) = &options.git_gpg_program {
            local.identity.git.gpg_program = gpg_program.clone();
        }
        if options.sign_commits {
            local.identity.git.sign_commits = true;
        }

        if !options.no_prompt {
            if local.identity.git.name.trim().is_empty() {
                local.identity.git.name = Input::new()
                    .with_prompt("Git user.name")
                    .interact_text()
                    .into_diagnostic()?;
            }
            if local.identity.git.email.trim().is_empty() {
                local.identity.git.email = Input::new()
                    .with_prompt("Git user.email")
                    .interact_text()
                    .into_diagnostic()?;
            }
            local.features.github = Confirm::new()
                .with_prompt("Enable GitHub feature?")
                .default(local.features.github)
                .interact()
                .into_diagnostic()?;
            local.features.terminal_apps = Confirm::new()
                .with_prompt("Enable terminal apps feature?")
                .default(local.features.terminal_apps)
                .interact()
                .into_diagnostic()?;
            local.features.git_lfs = Confirm::new()
                .with_prompt("Enable Git LFS feature?")
                .default(local.features.git_lfs)
                .interact()
                .into_diagnostic()?;
            local.features.ai_tools = Confirm::new()
                .with_prompt("Enable AI tools feature?")
                .default(local.features.ai_tools)
                .interact()
                .into_diagnostic()?;
            local.features.fastfetch = Confirm::new()
                .with_prompt("Enable fastfetch feature?")
                .default(local.features.fastfetch)
                .interact()
                .into_diagnostic()?;
        }

        Ok(local)
    }

    fn require_local_config(&self) -> Result<LocalConfig> {
        let path = self.local_config_path();
        if !path.is_file() {
            return Err(miette!(
                "missing local state: {}. Run 'dotctl bootstrap' first",
                path.display()
            ));
        }
        load_local_config(&path)
    }

    fn validate_local(&self, local: &LocalConfig, no_prompt: bool) -> Result<()> {
        if local.identity.git.name.trim().is_empty() {
            return Err(miette!(
                "git identity name is required{}",
                if no_prompt {
                    " (pass --git-name or run without --no-prompt)"
                } else {
                    ""
                }
            ));
        }
        if local.identity.git.email.trim().is_empty() {
            return Err(miette!(
                "git identity email is required{}",
                if no_prompt {
                    " (pass --git-email or run without --no-prompt)"
                } else {
                    ""
                }
            ));
        }
        Ok(())
    }

    fn resolve_state(&self, repo_root: &Path, local: LocalConfig) -> Result<ResolvedState> {
        let _profile: ProfileConfig = load_profile(repo_root, &local.profile)?;
        let pm = detect_package_manager(&self.runtime, &local.system.package_manager_override)?;
        let bundles = self.active_bundle_ids(repo_root, &local.features)?;
        Ok(ResolvedState {
            repo_root: repo_root.to_path_buf(),
            profile_name: local.profile.clone(),
            local,
            package_manager: pm.as_str().to_string(),
            platform: platform_name().to_string(),
            applied_bundles: bundles,
        })
    }

    fn active_bundle_ids(&self, repo_root: &Path, features: &FeatureFlags) -> Result<Vec<String>> {
        let mut ids = vec!["core".to_string(), "dev".to_string()];
        for bundle in load_bundle_specs(repo_root)? {
            if bundle.id == "core" || bundle.id == "dev" {
                continue;
            }
            match bundle.feature.as_deref() {
                Some("github") if features.github => ids.push(bundle.id),
                Some("terminal_apps") if features.terminal_apps => ids.push(bundle.id),
                Some("git_lfs") if features.git_lfs => ids.push(bundle.id),
                Some("ai_tools") if features.ai_tools => ids.push(bundle.id),
                Some("fastfetch") if features.fastfetch => ids.push(bundle.id),
                Some(_) | None => {}
            }
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    }

    fn install_packages(&self, resolved: &ResolvedState, upgrade: bool) -> Result<()> {
        let repo_root = &resolved.repo_root;
        let pm = PackageManagerId::parse(&resolved.package_manager)?;
        let bundles = selected_bundles(repo_root, &resolved.applied_bundles)?;
        let needs_root = matches!(
            pm,
            PackageManagerId::Apt | PackageManagerId::Dnf | PackageManagerId::Pacman
        );
        if needs_root {
            self.section("sudo authentication");
            self.runtime.ensure_sudo_access()?;
        }
        if pm == PackageManagerId::Apt {
            self.section("apt package index");
            self.runtime.run_root("apt-get", &["update"])?;
        }

        self.section("packages");
        for bundle in bundles {
            self.detail("bundle", &bundle.id);
            for group in &bundle.packages.required {
                if group.manager != resolved.package_manager {
                    continue;
                }
                for package in &group.names {
                    self.install_single_package(pm, package, true, upgrade)?;
                }
            }
            for group in &bundle.packages.optional {
                if group.manager != resolved.package_manager {
                    continue;
                }
                for package in &group.names {
                    self.install_single_package(pm, package, false, upgrade)?;
                }
            }
        }
        Ok(())
    }

    fn install_single_package(
        &self,
        pm: PackageManagerId,
        package: &str,
        required: bool,
        upgrade: bool,
    ) -> Result<()> {
        if upgrade
            && !pacman_selective_upgrade_supported(pm)
            && package_installed(&self.runtime, pm, package)?
        {
            self.status(
                "skip",
                &format!("{package} (managed by pacman; no selective upgrade)"),
            );
            return Ok(());
        }

        let already_available = if upgrade {
            package_installed(&self.runtime, pm, package)?
        } else {
            package_available(&self.runtime, pm, package)?
        };

        if already_available && !upgrade {
            self.status("skip", package);
            return Ok(());
        }

        if upgrade
            && package_command_available(&self.runtime, package)
            && !package_installed(&self.runtime, pm, package)?
        {
            self.status("skip", &format!("{package} (provided externally)"));
            return Ok(());
        }

        self.status(
            "run",
            &format!(
                "{} {}",
                if upgrade { "updating" } else { "installing" },
                package
            ),
        );
        let result = crate::system::install_package(&self.runtime, pm, package, upgrade);
        match (required, result) {
            (_, Ok(())) => {
                self.status("ok", package);
                Ok(())
            }
            (true, Err(err)) => Err(err),
            (false, Err(err)) => {
                self.status(
                    "warn",
                    &format!("optional package failed: {package} ({err})"),
                );
                Ok(())
            }
        }
    }

    fn install_managed_tools(&self, resolved: &ResolvedState, upgrade: bool) -> Result<()> {
        let registry = load_installer_registry(&resolved.repo_root)?;
        let tool_ids = selected_managed_tools(&resolved.repo_root, &resolved.applied_bundles)?;
        self.section("managed tools");
        for tool_id in tool_ids {
            self.install_managed_tool(&registry, &tool_id, upgrade)?;
        }
        Ok(())
    }

    fn install_managed_tool(
        &self,
        registry: &InstallerRegistry,
        tool_id: &str,
        upgrade: bool,
    ) -> Result<()> {
        let spec = registry
            .installers
            .get(tool_id)
            .ok_or_else(|| miette!("missing installer spec for {tool_id}"))?;
        match spec.kind.as_str() {
            "git" => self.ensure_git_tool(tool_id, spec, upgrade),
            "script" => self.ensure_script_tool(tool_id, spec, upgrade),
            "npm" => self.ensure_npm_tool(tool_id, spec, upgrade),
            "github_release" => self.ensure_github_release_tool(tool_id, spec),
            "package_manager" => Ok(()),
            other => Err(miette!(
                "unsupported installer kind '{other}' for {tool_id}"
            )),
        }
    }

    fn ensure_git_tool(&self, tool_id: &str, spec: &InstallerSpec, _upgrade: bool) -> Result<()> {
        let install_dir = expand_tilde(
            self.render_spec_value(
                spec.install_dir
                    .as_deref()
                    .ok_or_else(|| miette!("missing install_dir for {tool_id}"))?,
            )
            .as_str(),
            &self.runtime.home,
        );
        let repo = spec
            .repo
            .as_deref()
            .ok_or_else(|| miette!("missing repo for {tool_id}"))?;
        if install_dir.join(".git").is_dir() {
            self.status(
                "run",
                &format!("updating {tool_id} at {}", install_dir.display()),
            );
            let status = self
                .runtime
                .command("git")
                .args([
                    "-C",
                    install_dir.to_string_lossy().as_ref(),
                    "pull",
                    "--ff-only",
                ])
                .status()
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to update {tool_id}"))?;
            if !status.success() {
                return Err(miette!("failed to update {tool_id}"));
            }
        } else {
            self.status(
                "run",
                &format!("installing {tool_id} at {}", install_dir.display()),
            );
            fs::create_dir_all(install_dir.parent().unwrap_or_else(|| Path::new(".")))
                .into_diagnostic()?;
            let status = self
                .runtime
                .command("git")
                .args([
                    "clone",
                    "--depth=1",
                    repo,
                    install_dir.to_string_lossy().as_ref(),
                ])
                .status()
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to clone {tool_id}"))?;
            if !status.success() {
                return Err(miette!("failed to install {tool_id}"));
            }
        }
        self.status("ok", tool_id);
        Ok(())
    }

    fn ensure_script_tool(&self, tool_id: &str, spec: &InstallerSpec, upgrade: bool) -> Result<()> {
        let binary = spec.binary.as_deref().unwrap_or(tool_id);
        if self.runtime.has_cmd(binary) {
            if !upgrade {
                self.status("skip", tool_id);
                return Ok(());
            }
            if self.run_self_update(tool_id)? {
                self.status("ok", tool_id);
                return Ok(());
            }
        }
        let action = if upgrade {
            format!("updating {tool_id}")
        } else {
            format!("installing {tool_id}")
        };
        self.status("run", &action);
        let interpreter = spec.interpreter.as_deref().unwrap_or("sh");
        let args = spec
            .args
            .iter()
            .map(|value| self.render_spec_value(value))
            .collect::<Vec<_>>();
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let env_pairs = spec
            .env
            .iter()
            .map(|(key, value)| (key.clone(), self.render_spec_value(value)))
            .collect::<Vec<_>>();
        let env_refs = env_pairs
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        let script_name = format!("{tool_id}-install.sh");
        self.runtime.run_downloaded_script(
            spec.url
                .as_deref()
                .ok_or_else(|| miette!("missing url for {tool_id}"))?,
            &script_name,
            interpreter,
            &arg_refs,
            &env_refs,
        )?;
        self.status("ok", tool_id);
        Ok(())
    }

    fn ensure_npm_tool(&self, tool_id: &str, spec: &InstallerSpec, upgrade: bool) -> Result<()> {
        let binary = spec.binary.as_deref().unwrap_or(tool_id);
        if self.runtime.has_cmd(binary) && !upgrade {
            self.status("skip", tool_id);
            return Ok(());
        }
        let prefix = self.render_spec_value(
            spec.prefix
                .as_deref()
                .ok_or_else(|| miette!("missing prefix for {tool_id}"))?,
        );
        fs::create_dir_all(PathBuf::from(&prefix).join("bin")).into_diagnostic()?;
        let package = spec
            .package
            .as_deref()
            .ok_or_else(|| miette!("missing npm package for {tool_id}"))?;
        let package = if upgrade && spec.update_policy.as_deref() == Some("latest") {
            format!("{package}@latest")
        } else {
            package.to_string()
        };
        let action = if upgrade {
            format!("updating {tool_id}")
        } else {
            format!("installing {tool_id}")
        };
        self.status("run", &action);
        self.runtime.run_checked_with_env(
            "npm",
            &["i", "-g", package.as_str()],
            &[("npm_config_prefix", prefix.as_str())],
        )?;
        self.status("ok", tool_id);
        Ok(())
    }

    fn ensure_github_release_tool(&self, tool_id: &str, spec: &InstallerSpec) -> Result<()> {
        match spec.asset_strategy.as_deref() {
            Some("neovim") => self.ensure_neovim(tool_id, spec),
            Some("yazi") => self.ensure_yazi(tool_id, spec),
            Some(other) => Err(miette!(
                "unsupported asset strategy '{other}' for {tool_id}"
            )),
            None => Err(miette!("missing asset_strategy for {tool_id}")),
        }
    }

    fn run_self_update(&self, tool_id: &str) -> Result<bool> {
        match tool_id {
            "uv" => {
                self.status("run", "updating uv");
                self.runtime.run_checked("uv", &["self", "update"])?;
                Ok(true)
            }
            "rustup" => {
                self.status("run", "updating rustup");
                self.runtime.run_checked("rustup", &["self", "update"])?;
                self.runtime.run_checked("rustup", &["update"])?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn ensure_neovim(&self, tool_id: &str, spec: &InstallerSpec) -> Result<()> {
        let repo = spec
            .repo
            .as_deref()
            .ok_or_else(|| miette!("missing repo for {tool_id}"))?;
        let release = self.runtime.latest_github_release(repo)?;
        let asset_name = detect_github_release_asset(spec)?;
        let asset = find_asset(&release, &asset_name)?;
        let expected_sha = asset
            .digest_sha256
            .clone()
            .ok_or_else(|| miette!("missing SHA-256 digest for {}", asset_name))?;
        let version_dir = self
            .runtime
            .home
            .join(".local/opt")
            .join(format!("nvim-{}", release.tag));
        let stable_link = self.runtime.home.join(".local/opt/nvim-stable");
        let bin_link = self.runtime.home.join(".local/bin/nvim");
        if let Some(current) = self.current_neovim_version()? {
            if current == release.tag && version_dir.join("bin/nvim").is_file() {
                self.symlink_force(&version_dir, &stable_link)?;
                self.symlink_force(&version_dir.join("bin/nvim"), &bin_link)?;
                self.status("skip", &format!("Neovim already up to date ({current})"));
                return Ok(());
            }
        }

        fs::create_dir_all(self.runtime.home.join(".local/opt")).into_diagnostic()?;
        fs::create_dir_all(self.runtime.home.join(".local/bin")).into_diagnostic()?;
        let tmp = tempfile::tempdir().into_diagnostic()?;
        let archive = tmp.path().join(&asset_name);
        let staging = tmp.path().join("nvim");
        self.status("run", &format!("downloading {}", asset.download_url));
        self.runtime.download_file(&asset.download_url, &archive)?;
        let actual = self.runtime.sha256_file(&archive)?;
        if spec.verify_checksum.unwrap_or(false) && actual != expected_sha {
            return Err(miette!(
                "checksum mismatch for {}: expected {}, got {}",
                asset_name,
                expected_sha,
                actual
            ));
        }
        fs::create_dir_all(&staging).into_diagnostic()?;
        let status = self
            .runtime
            .command("tar")
            .args([
                "-xzf",
                archive.to_string_lossy().as_ref(),
                "-C",
                staging.to_string_lossy().as_ref(),
                "--strip-components=1",
            ])
            .status()
            .into_diagnostic()
            .wrap_err("failed to extract Neovim archive")?;
        if !status.success() {
            return Err(miette!("failed to extract Neovim archive"));
        }
        if !staging.join("bin/nvim").is_file() {
            return Err(miette!(
                "downloaded Neovim archive did not contain bin/nvim"
            ));
        }
        if version_dir.exists() {
            fs::remove_dir_all(&version_dir).into_diagnostic()?;
        }
        fs::rename(&staging, &version_dir).into_diagnostic()?;
        self.symlink_force(&version_dir, &stable_link)?;
        self.symlink_force(&version_dir.join("bin/nvim"), &bin_link)?;
        self.status("ok", &format!("Neovim installed ({})", release.tag));
        Ok(())
    }

    fn ensure_yazi(&self, tool_id: &str, spec: &InstallerSpec) -> Result<()> {
        let repo = spec
            .repo
            .as_deref()
            .ok_or_else(|| miette!("missing repo for {tool_id}"))?;
        let release = self.runtime.latest_github_release(repo)?;
        let asset_name = detect_github_release_asset(spec)?;
        let asset = find_asset(&release, &asset_name)?;
        let expected_sha = asset
            .digest_sha256
            .clone()
            .ok_or_else(|| miette!("missing SHA-256 digest for {}", asset_name))?;
        let latest_version = release.tag.trim_start_matches('v').to_string();
        let version_dir = self
            .runtime
            .home
            .join(".local/opt")
            .join(format!("yazi-{latest_version}"));
        let stable_link = self.runtime.home.join(".local/opt/yazi-stable");
        let yazi_link = self.runtime.home.join(".local/bin/yazi");
        let ya_link = self.runtime.home.join(".local/bin/ya");
        if let Some(current) = self.current_yazi_version()? {
            if current == latest_version && version_dir.join("yazi").is_file() {
                self.symlink_force(&version_dir, &stable_link)?;
                self.symlink_force(&version_dir.join("yazi"), &yazi_link)?;
                self.symlink_force(&version_dir.join("ya"), &ya_link)?;
                self.status("skip", &format!("Yazi already up to date ({current})"));
                return Ok(());
            }
        }
        fs::create_dir_all(self.runtime.home.join(".local/opt")).into_diagnostic()?;
        fs::create_dir_all(self.runtime.home.join(".local/bin")).into_diagnostic()?;
        let tmp = tempfile::tempdir().into_diagnostic()?;
        let archive = tmp.path().join(&asset_name);
        let staging = tmp.path().join("yazi");
        self.status("run", &format!("downloading {}", asset.download_url));
        self.runtime.download_file(&asset.download_url, &archive)?;
        let actual = self.runtime.sha256_file(&archive)?;
        if spec.verify_checksum.unwrap_or(false) && actual != expected_sha {
            return Err(miette!(
                "checksum mismatch for {}: expected {}, got {}",
                asset_name,
                expected_sha,
                actual
            ));
        }
        fs::create_dir_all(&staging).into_diagnostic()?;
        let status = self
            .runtime
            .command("unzip")
            .args([
                "-q",
                archive.to_string_lossy().as_ref(),
                "-d",
                staging.to_string_lossy().as_ref(),
            ])
            .status()
            .into_diagnostic()
            .wrap_err("failed to extract Yazi archive")?;
        if !status.success() {
            return Err(miette!("failed to extract Yazi archive"));
        }
        let extracted_dir = first_subdir(&staging)?;
        if !extracted_dir.join("yazi").is_file() || !extracted_dir.join("ya").is_file() {
            return Err(miette!(
                "downloaded Yazi archive did not contain yazi/ya binaries"
            ));
        }
        if version_dir.exists() {
            fs::remove_dir_all(&version_dir).into_diagnostic()?;
        }
        fs::rename(&extracted_dir, &version_dir).into_diagnostic()?;
        self.symlink_force(&version_dir, &stable_link)?;
        self.symlink_force(&version_dir.join("yazi"), &yazi_link)?;
        self.symlink_force(&version_dir.join("ya"), &ya_link)?;
        self.status("ok", &format!("Yazi installed ({latest_version})"));
        Ok(())
    }

    fn current_neovim_version(&self) -> Result<Option<String>> {
        let bin = self.runtime.home.join(".local/bin/nvim");
        if !bin.is_file() {
            return Ok(None);
        }
        let out = self.runtime.capture_stdout("nvim", &["--version"])?;
        Ok(out
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .map(|s| s.to_string()))
    }

    fn current_yazi_version(&self) -> Result<Option<String>> {
        let bin = self.runtime.home.join(".local/bin/yazi");
        if !bin.is_file() {
            return Ok(None);
        }
        let out = self.runtime.capture_stdout("yazi", &["--version"])?;
        Ok(out.split_whitespace().nth(1).map(|s| s.to_string()))
    }

    fn generate_chezmoi_config(&self, resolved: &ResolvedState) -> Result<()> {
        let config = ChezmoiConfig {
            source_dir: &resolved.local.repo.source_dir,
            data: ChezmoiData {
                repo: &resolved.local.repo,
                features: &resolved.local.features,
                identity: &resolved.local.identity,
            },
        };
        let raw = toml::to_string_pretty(&config).into_diagnostic()?;
        write_secure_file(&self.chezmoi_config_path(), &raw)
    }

    fn ensure_local_extension_points(&self) -> Result<()> {
        self.ensure_stub_file(
            self.runtime.home.join(".gitconfig.extra"),
            "# Unmanaged local Git config\n",
        )?;
        self.ensure_stub_file(
            self.runtime.home.join(".zprofile.local"),
            "# Unmanaged local login-shell overrides\n",
        )?;
        self.ensure_stub_file(
            self.runtime.home.join(".zshrc.local"),
            "# Unmanaged local interactive zsh overrides\n",
        )?;
        Ok(())
    }

    fn apply_chezmoi(&self) -> Result<()> {
        self.section("chezmoi apply");
        self.runtime.run_checked("chezmoi", &["apply"])
    }

    fn run_post_apply_tasks(&self, resolved: &ResolvedState, update_mode: bool) -> Result<()> {
        let mut tasks = selected_post_apply_tasks(&resolved.repo_root, &resolved.applied_bundles)?;
        tasks.insert("nvim_sync".to_string());

        for task in tasks {
            match task.as_str() {
                "zsh_bundle" => self.sync_zsh_plugins()?,
                "yazi_sync" => self.sync_yazi_packages(update_mode)?,
                "nvim_sync" => self.sync_neovim(update_mode)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn run_minimal_post_apply_tasks(&self, resolved: &ResolvedState) -> Result<()> {
        let tasks = selected_post_apply_tasks(&resolved.repo_root, &resolved.applied_bundles)?;
        if tasks.contains("zsh_bundle") {
            self.sync_zsh_plugins()?;
        }
        Ok(())
    }

    fn sync_zsh_plugins(&self) -> Result<()> {
        let manifest = self.runtime.home.join(".zsh_plugins.txt");
        let cache_dir = self.runtime.home.join(".cache/zsh");
        let bundle = cache_dir.join(".zsh_plugins.zsh");
        let antidote = self.runtime.home.join(".antidote/antidote.zsh");
        if !manifest.is_file() || !antidote.is_file() {
            return Ok(());
        }
        fs::create_dir_all(&cache_dir).into_diagnostic()?;
        let tmp = NamedTempFile::new_in(&cache_dir).into_diagnostic()?;
        self.section("zsh plugin bundle");
        let script = format!(
            "#!/usr/bin/env zsh
set -eu
set +u
source {}
set -u
antidote update --bundles
antidote bundle < {} > {}
",
            shell_escape(antidote.to_string_lossy().as_ref()),
            shell_escape(manifest.to_string_lossy().as_ref()),
            shell_escape(tmp.path().to_string_lossy().as_ref()),
        );
        let script_file = NamedTempFile::new_in(&cache_dir).into_diagnostic()?;
        fs::write(script_file.path(), script).into_diagnostic()?;
        let status = self
            .runtime
            .command("zsh")
            .arg(script_file.path())
            .status()
            .into_diagnostic()
            .wrap_err("failed to build zsh plugin bundle")?;
        if !status.success() {
            return Err(miette!("zsh plugin bundle build failed"));
        }
        tmp.persist(&bundle)
            .into_diagnostic()
            .wrap_err("failed to persist zsh plugin bundle")?;
        self.status("ok", &format!("zsh plugins  {}", bundle.display()));
        Ok(())
    }

    fn sync_yazi_packages(&self, update_mode: bool) -> Result<()> {
        if !self.runtime.has_cmd("ya") {
            return Ok(());
        }
        let package_file = self.runtime.home.join(".config/yazi/package.toml");
        if !package_file.is_file() {
            return Ok(());
        }
        let raw = fs::read_to_string(&package_file).into_diagnostic()?;
        if !raw.contains("[[plugin.deps]]") && !raw.contains("[[flavor.deps]]") {
            return Ok(());
        }
        self.section("yazi package sync");
        let action = if update_mode { "upgrade" } else { "install" };
        self.runtime.run_checked("ya", &["pkg", action])
    }

    fn sync_neovim(&self, update_mode: bool) -> Result<()> {
        let init = self.runtime.home.join(".config/nvim/init.lua");
        if !init.is_file() {
            return Err(miette!(
                "Neovim config is not present in ~/.config/nvim. Run 'dotctl apply' first."
            ));
        }
        self.section("neovim sync");
        self.require_command("nvim")?;
        self.require_command("uv")?;
        self.require_command("cargo")?;
        self.require_command("node")?;
        self.require_command("npm")?;
        self.require_command("git")?;
        self.require_command("curl")?;
        self.require_command("tar")?;
        self.require_command("unzip")?;
        self.require_command("make")?;
        self.require_command("cc")?;

        if update_mode || !self.uv_tool_installed("pynvim")? {
            self.runtime.run_checked(
                "uv",
                &[
                    "tool",
                    "install",
                    if update_mode { "--upgrade" } else { "pynvim" },
                    if update_mode { "pynvim" } else { "" },
                ]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
            )?;
        }
        if update_mode || !self.cargo_package_installed("tree-sitter-cli")? {
            let args = if update_mode {
                vec!["install", "tree-sitter-cli", "--locked", "--force"]
            } else {
                vec!["install", "tree-sitter-cli", "--locked"]
            };
            self.runtime.run_checked("cargo", &args)?;
        }
        self.run_nvim_headless(&["+Lazy! sync", "+Lazy! clean", "+qall"])?;
        let languages = self
            .run_nvim_headless_capture(&[
                "+lua require(\"config.bootstrap_tasks\").print_treesitter_languages()",
                "+qall",
            ])?
            .trim()
            .to_string();
        if !languages.is_empty() {
            self.run_nvim_headless(&[
                "+lua require(\"config.bootstrap_tasks\").ensure_treesitter_parsers()",
                "+qall",
            ])?;
        }
        let mason = self
            .run_nvim_headless_capture(&[
                "+lua require(\"config.bootstrap_tasks\").print_mason_packages()",
                "+qall",
            ])?
            .trim()
            .to_string();
        if !mason.is_empty() {
            self.run_nvim_headless(&["-c", "MasonUpdate", "-c", "qall"])?;
            self.run_nvim_headless(&[
                "+lua require(\"config.bootstrap_tasks\").ensure_mason_packages({ update = true })",
                "+qall",
            ])?;
        }
        self.run_nvim_headless(&["+qall"])?;
        self.status("ok", "Neovim config loaded successfully");
        Ok(())
    }

    fn run_doctor(&self, resolved: &ResolvedState) -> Result<bool> {
        self.section("doctor");
        let mut ok = true;
        let required = ["git", "chezmoi", "zsh", "curl", "nvim"];
        let convenience = ["fzf", "zoxide", "eza"];
        let dev = [
            "cc", "make", "uv", "rustup", "cargo", "rustc", "node", "npm",
        ];
        for cmd in required {
            ok &= self.check_command(cmd, true);
        }
        for cmd in convenience {
            self.check_command(cmd, false);
        }
        for cmd in dev {
            ok &= self.check_command(cmd, true);
        }
        if resolved.local.features.github {
            ok &= self.check_command("gh", true);
        }
        if resolved.local.features.git_lfs {
            ok &= self.check_command("git-lfs", true);
        }
        if resolved.local.features.terminal_apps {
            ok &= self.check_command("tmux", true);
            ok &= self.check_command("btop", true);
            ok &= self.check_command("starship", true);
            ok &= self.check_command("yazi", true);
            ok &= self.check_command("ya", true);
        }
        if resolved.local.features.ai_tools {
            ok &= self.check_command("codex", true);
            ok &= self.check_command("claude", true);
        }
        if self
            .runtime
            .command("chezmoi")
            .arg("verify")
            .status()
            .into_diagnostic()?
            .success()
        {
            self.status("ok", "chezmoi verify");
        } else {
            self.status("fail", "chezmoi verify");
            ok = false;
        }
        if self
            .runtime
            .command("zsh")
            .args(["-ic", "true"])
            .status()
            .into_diagnostic()?
            .success()
        {
            self.status("ok", "zsh interactive startup");
        } else {
            self.status("fail", "zsh interactive startup");
            ok = false;
        }
        if self
            .runtime
            .command("nvim")
            .args(["--headless", "+qall"])
            .status()
            .into_diagnostic()?
            .success()
        {
            self.status("ok", "nvim headless load");
        } else {
            self.status("fail", "nvim headless load");
            ok = false;
        }
        Ok(ok)
    }

    fn write_snapshot(&self, resolved: &ResolvedState) -> Result<()> {
        let snapshot = StateSnapshot {
            profile: resolved.profile_name.clone(),
            repo: resolved.local.repo.clone(),
            features: resolved.local.features.clone(),
            identity: resolved.local.identity.clone(),
            system: resolved.local.system.clone(),
            platform: resolved.platform.clone(),
            package_manager: resolved.package_manager.clone(),
            applied_bundles: resolved.applied_bundles.clone(),
            last_applied_at: Local::now().to_rfc3339(),
        };
        write_snapshot(&self.snapshot_path(), &snapshot)
    }

    fn ensure_stub_file(&self, path: PathBuf, header: &str) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        write_secure_file(&path, header)
    }

    fn local_config_path(&self) -> PathBuf {
        self.runtime.home.join(".config/dotfiles/local.toml")
    }

    fn snapshot_path(&self) -> PathBuf {
        self.runtime.home.join(".config/dotfiles/state.toml")
    }

    fn chezmoi_config_path(&self) -> PathBuf {
        self.runtime.home.join(".config/chezmoi/chezmoi.toml")
    }

    fn require_command(&self, command: &str) -> Result<()> {
        if self.runtime.has_cmd(command) {
            Ok(())
        } else {
            Err(miette!("missing required command: {command}"))
        }
    }

    fn render_spec_value(&self, value: &str) -> String {
        value.replace("{{home}}", self.runtime.home.to_string_lossy().as_ref())
    }

    fn check_command(&self, command: &str, required: bool) -> bool {
        if self.runtime.has_cmd(command) {
            self.status(
                "ok",
                &format!(
                    "{} {}",
                    pad_command(command),
                    which_path(&self.runtime, command)
                ),
            );
            true
        } else {
            self.status(if required { "miss" } else { "note" }, command);
            !required
        }
    }

    fn uv_tool_installed(&self, tool: &str) -> Result<bool> {
        let out = self.runtime.capture_stdout("uv", &["tool", "list"])?;
        Ok(out
            .lines()
            .any(|line| line.split_whitespace().next() == Some(tool)))
    }

    fn cargo_package_installed(&self, package: &str) -> Result<bool> {
        let out = self
            .runtime
            .capture_stdout("cargo", &["install", "--list"])?;
        Ok(out
            .lines()
            .any(|line| line.starts_with(&format!("{package} v"))))
    }

    fn run_nvim_headless(&self, args: &[&str]) -> Result<()> {
        let status = self
            .runtime
            .command("nvim")
            .arg("--headless")
            .args(args)
            .status()
            .into_diagnostic()
            .wrap_err("failed to run Neovim headless")?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("Neovim headless command failed"))
        }
    }

    fn run_nvim_headless_capture(&self, args: &[&str]) -> Result<String> {
        let output = self
            .runtime
            .command("nvim")
            .arg("--headless")
            .args(args)
            .output()
            .into_diagnostic()
            .wrap_err("failed to run Neovim headless")?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(miette!(
                String::from_utf8_lossy(&output.stderr).into_owned()
            ))
        }
    }

    fn symlink_force(&self, target: &Path, link: &Path) -> Result<()> {
        if link.exists() || link.symlink_metadata().is_ok() {
            if link.is_dir() && !link.is_symlink() {
                fs::remove_dir_all(link).into_diagnostic()?;
            } else {
                fs::remove_file(link).into_diagnostic()?;
            }
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create symlink {}", link.display()))?;
        #[cfg(not(unix))]
        std::os::windows::fs::symlink_file(target, link)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create symlink {}", link.display()))?;
        Ok(())
    }

    fn section(&self, title: &str) {
        println!("{}", self.ui.section(title));
    }

    fn detail(&self, label: &str, value: &str) {
        println!("{}", self.ui.detail(label, value));
    }

    fn status(&self, label: &str, message: &str) {
        println!("{}", self.ui.status(label, message));
    }

    fn summary(&self, title: &str, ok: bool) {
        println!("{}", self.ui.summary(title, ok));
    }
}

fn canonical_repo_root(repo: Option<PathBuf>) -> Result<PathBuf> {
    let path = repo.unwrap_or(std::env::current_dir().into_diagnostic()?);
    path.canonicalize()
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve repo path {}", path.display()))
}

fn selected_bundles(repo_root: &Path, ids: &[String]) -> Result<Vec<BundleSpec>> {
    let wanted: BTreeSet<_> = ids.iter().cloned().collect();
    let bundles = load_bundle_specs(repo_root)?;
    Ok(bundles
        .into_iter()
        .filter(|bundle| wanted.contains(&bundle.id))
        .collect())
}

fn selected_managed_tools(repo_root: &Path, ids: &[String]) -> Result<Vec<String>> {
    let mut tools = BTreeSet::new();
    for bundle in selected_bundles(repo_root, ids)? {
        for tool in bundle.tools.managed {
            tools.insert(tool);
        }
    }
    Ok(tools.into_iter().collect())
}

fn selected_post_apply_tasks(repo_root: &Path, ids: &[String]) -> Result<BTreeSet<String>> {
    let mut tasks = BTreeSet::new();
    for bundle in selected_bundles(repo_root, ids)? {
        for task in bundle.tools.post_apply {
            tasks.insert(task);
        }
    }
    Ok(tasks)
}

fn detect_github_release_asset(spec: &InstallerSpec) -> Result<String> {
    match spec.asset_strategy.as_deref() {
        Some("neovim") => {
            let platform = match std::env::consts::OS {
                "macos" => "macos",
                "linux" => "linux",
                other => {
                    return Err(miette!(
                        "unsupported operating system for Neovim bootstrap: {other}"
                    ));
                }
            };
            let arch = match std::env::consts::ARCH {
                "x86_64" => "x86_64",
                "aarch64" => "arm64",
                other => {
                    return Err(miette!(
                        "unsupported architecture for Neovim bootstrap: {other}"
                    ));
                }
            };
            Ok(format!("nvim-{platform}-{arch}.tar.gz"))
        }
        Some("yazi") => {
            let platform = match std::env::consts::OS {
                "macos" => "apple-darwin",
                "linux" => "unknown-linux-gnu",
                other => {
                    return Err(miette!(
                        "unsupported operating system for Yazi bootstrap: {other}"
                    ));
                }
            };
            let arch = match std::env::consts::ARCH {
                "x86_64" => "x86_64",
                "aarch64" => "aarch64",
                other => {
                    return Err(miette!(
                        "unsupported architecture for Yazi bootstrap: {other}"
                    ));
                }
            };
            Ok(format!("yazi-{arch}-{platform}.zip"))
        }
        Some(other) => Err(miette!("unsupported asset strategy: {other}")),
        None => Err(miette!("missing asset strategy")),
    }
}

fn find_asset<'a>(release: &'a GitHubRelease, name: &str) -> Result<&'a GitHubAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .ok_or_else(|| miette!("missing asset {} in release {}", name, release.tag))
}

fn first_subdir(path: &Path) -> Result<PathBuf> {
    for entry in fs::read_dir(path).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            return Ok(entry_path);
        }
    }
    Err(miette!("archive did not contain an extracted directory"))
}

fn pad_command(command: &str) -> String {
    format!("{command:<12}")
}

fn which_path(runtime: &Runtime, command: &str) -> String {
    which::which_in(command, Some(&runtime.path), &runtime.home)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "<missing>".to_string())
}

fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let escaped = value.replace('"', "\\\"");
    format!("\"{escaped}\"")
}
