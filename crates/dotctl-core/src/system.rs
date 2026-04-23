use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use miette::{Context, IntoDiagnostic, Result, miette};
use serde_json::Value;
use tempfile::TempDir;
use which::which_in;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManagerId {
    Brew,
    Apt,
    Dnf,
    Pacman,
}

impl PackageManagerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brew => "brew",
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "brew" => Ok(Self::Brew),
            "apt" => Ok(Self::Apt),
            "dnf" => Ok(Self::Dnf),
            "pacman" => Ok(Self::Pacman),
            other => Err(miette!("unsupported package manager: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Runtime {
    pub home: PathBuf,
    pub path: OsString,
}

#[derive(Debug, Clone)]
pub struct GitHubAsset {
    pub name: String,
    pub download_url: String,
    pub digest_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubRelease {
    pub tag: String,
    pub assets: Vec<GitHubAsset>,
}

impl Runtime {
    pub fn detect() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| miette!("could not resolve home directory"))?;
        Ok(Self::new(home.clone(), augmented_path(&home)))
    }

    pub fn new(home: PathBuf, path: OsString) -> Self {
        Self { home, path }
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.env("HOME", &self.home);
        command.env("PATH", &self.path);
        command
    }

    pub fn has_cmd(&self, program: &str) -> bool {
        which_in(program, Some(&self.path), &self.home).is_ok()
    }

    pub fn resolve_brew(&self) -> Option<PathBuf> {
        if let Ok(path) = which_in("brew", Some(&self.path), &self.home) {
            return Some(path);
        }
        for candidate in [
            "/opt/homebrew/bin/brew",
            "/usr/local/bin/brew",
            "/home/linuxbrew/.linuxbrew/bin/brew",
        ] {
            let path = PathBuf::from(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    pub fn capture_stdout(&self, program: &str, args: &[&str]) -> Result<String> {
        let output = self
            .command(program)
            .args(args)
            .output()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to run {}", program))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(command_failed(program, args, &output.stderr))
        }
    }

    pub fn run_checked(&self, program: &str, args: &[&str]) -> Result<()> {
        let status = self
            .command(program)
            .args(args)
            .status()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to run {}", program))?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("command failed: {} {}", program, args.join(" ")))
        }
    }

    pub fn run_checked_with_env(
        &self,
        program: &str,
        args: &[&str],
        envs: &[(&str, &str)],
    ) -> Result<()> {
        let mut command = self.command(program);
        command.args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        let status = command
            .status()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to run {}", program))?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("command failed: {} {}", program, args.join(" ")))
        }
    }

    pub fn ensure_sudo_access(&self) -> Result<()> {
        if is_root()? {
            return Ok(());
        }
        if !self.has_cmd("sudo") {
            return Err(miette!("sudo is required for the detected package manager"));
        }
        let status = self
            .command("sudo")
            .arg("-v")
            .status()
            .into_diagnostic()
            .wrap_err("failed to request sudo authentication")?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("sudo authentication failed"))
        }
    }

    pub fn run_root(&self, program: &str, args: &[&str]) -> Result<()> {
        if is_root()? {
            return self.run_checked(program, args);
        }
        let status = self
            .command("sudo")
            .arg(program)
            .args(args)
            .status()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to run sudo {}", program))?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!(
                "command failed: sudo {} {}",
                program,
                args.join(" ")
            ))
        }
    }

    pub fn download_text(&self, url: &str) -> Result<String> {
        let output = self
            .command("curl")
            .args(["-fsSL", "--retry", "3", "--retry-delay", "1", url])
            .output()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to download {}", url))?;
        if !output.status.success() {
            return Err(command_failed("curl", &[url], &output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    pub fn download_file(&self, url: &str, output: &Path) -> Result<()> {
        let status = self
            .command("curl")
            .args([
                "-fL",
                "--retry",
                "3",
                "--retry-delay",
                "1",
                "--output",
                output.to_string_lossy().as_ref(),
                url,
            ])
            .status()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to download {}", url))?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("failed to download {}", url))
        }
    }

    pub fn run_downloaded_script(
        &self,
        url: &str,
        script_name: &str,
        interpreter: &str,
        args: &[&str],
        envs: &[(&str, &str)],
    ) -> Result<()> {
        let tmp = TempDir::new().into_diagnostic()?;
        let script = tmp.path().join(script_name);
        self.download_file(url, &script)?;
        let mut command = self.command(interpreter);
        command.arg(&script).args(args);
        for (key, value) in envs {
            command.env(key, value);
        }
        let status = command
            .status()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to run downloaded script {}", url))?;
        if status.success() {
            Ok(())
        } else {
            Err(miette!("downloaded script failed: {}", url))
        }
    }

    pub fn latest_github_release(&self, repo: &str) -> Result<GitHubRelease> {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let output = self
            .command("curl")
            .args([
                "-fsSL",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "X-GitHub-Api-Version: 2022-11-28",
                &url,
            ])
            .output()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to query latest release for {repo}"))?;
        if !output.status.success() {
            return Err(command_failed("curl", &[&url], &output.stderr));
        }
        let value: Value = serde_json::from_slice(&output.stdout)
            .into_diagnostic()
            .wrap_err("failed to parse GitHub release JSON")?;
        let tag = value
            .get("tag_name")
            .and_then(Value::as_str)
            .ok_or_else(|| miette!("missing tag_name in GitHub release response for {repo}"))?
            .to_string();
        let assets = value
            .get("assets")
            .and_then(Value::as_array)
            .ok_or_else(|| miette!("missing assets in GitHub release response for {repo}"))?
            .iter()
            .filter_map(|asset| {
                let name = asset.get("name")?.as_str()?.to_string();
                let download_url = asset.get("browser_download_url")?.as_str()?.to_string();
                let digest_sha256 = asset
                    .get("digest")
                    .and_then(Value::as_str)
                    .and_then(|digest| digest.strip_prefix("sha256:"))
                    .map(ToOwned::to_owned);
                Some(GitHubAsset {
                    name,
                    download_url,
                    digest_sha256,
                })
            })
            .collect();
        Ok(GitHubRelease { tag, assets })
    }

    pub fn sha256_file(&self, path: &Path) -> Result<String> {
        if self.has_cmd("sha256sum") {
            return self
                .capture_stdout("sha256sum", &[path.to_string_lossy().as_ref()])
                .map(|raw| {
                    raw.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string()
                });
        }
        if self.has_cmd("shasum") {
            return self
                .capture_stdout("shasum", &["-a", "256", path.to_string_lossy().as_ref()])
                .map(|raw| {
                    raw.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string()
                });
        }
        if self.has_cmd("openssl") {
            return self
                .capture_stdout(
                    "openssl",
                    &["dgst", "-sha256", path.to_string_lossy().as_ref()],
                )
                .map(|raw| {
                    raw.split_whitespace()
                        .last()
                        .unwrap_or_default()
                        .to_string()
                });
        }
        Err(miette!(
            "could not find a SHA-256 tool (sha256sum, shasum, or openssl)"
        ))
    }
}

pub fn detect_package_manager(runtime: &Runtime, override_pm: &str) -> Result<PackageManagerId> {
    if !override_pm.trim().is_empty() {
        return PackageManagerId::parse(override_pm.trim());
    }

    match std::env::consts::OS {
        "macos" => {
            if runtime.resolve_brew().is_some() {
                return Ok(PackageManagerId::Brew);
            }
            Err(miette!("Homebrew is required on macOS"))
        }
        "linux" => {
            if runtime.has_cmd("apt-get") {
                return Ok(PackageManagerId::Apt);
            }
            if runtime.has_cmd("dnf") {
                return Ok(PackageManagerId::Dnf);
            }
            if runtime.has_cmd("pacman") {
                return Ok(PackageManagerId::Pacman);
            }
            if runtime.resolve_brew().is_some() {
                return Ok(PackageManagerId::Brew);
            }
            Err(miette!("could not detect a supported package manager"))
        }
        other => Err(miette!("unsupported operating system: {other}")),
    }
}

pub fn platform_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    }
}

pub fn command_for_package(package: &str) -> &str {
    match package {
        "neovim" => "nvim",
        "github-cli" => "gh",
        "nodejs" => "node",
        other => other,
    }
}

pub fn package_command_available(runtime: &Runtime, package: &str) -> bool {
    runtime.has_cmd(command_for_package(package))
}

pub fn package_installed(runtime: &Runtime, pm: PackageManagerId, package: &str) -> Result<bool> {
    let installed = match pm {
        PackageManagerId::Apt => runtime
            .command("dpkg-query")
            .args(["-W", "-f=${Status}", package])
            .output()
            .into_diagnostic()?
            .status
            .success(),
        PackageManagerId::Dnf => runtime
            .command("rpm")
            .args(["-q", package])
            .output()
            .into_diagnostic()?
            .status
            .success(),
        PackageManagerId::Pacman => runtime
            .command("pacman")
            .args(["-Q", package])
            .output()
            .into_diagnostic()?
            .status
            .success(),
        PackageManagerId::Brew => {
            let Some(brew) = runtime.resolve_brew() else {
                return Ok(false);
            };
            Command::new(brew)
                .arg("list")
                .arg("--formula")
                .arg(package)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .into_diagnostic()?
                .success()
        }
    };
    Ok(installed)
}

pub fn package_available(runtime: &Runtime, pm: PackageManagerId, package: &str) -> Result<bool> {
    if package_command_available(runtime, package) {
        return Ok(true);
    }
    package_installed(runtime, pm, package)
}

pub fn install_package(
    runtime: &Runtime,
    pm: PackageManagerId,
    package: &str,
    upgrade: bool,
) -> Result<()> {
    match pm {
        PackageManagerId::Apt => {
            if upgrade && package_installed(runtime, pm, package)? {
                runtime.run_root("apt-get", &["install", "--only-upgrade", "-y", package])
            } else {
                runtime.run_root("apt-get", &["install", "-y", package])
            }
        }
        PackageManagerId::Dnf => {
            if upgrade && package_installed(runtime, pm, package)? {
                runtime.run_root("dnf", &["upgrade", "-y", package])
            } else {
                runtime.run_root("dnf", &["install", "-y", package])
            }
        }
        PackageManagerId::Pacman => {
            runtime.run_root("pacman", &["-S", "--needed", "--noconfirm", package])
        }
        PackageManagerId::Brew => {
            let brew = runtime
                .resolve_brew()
                .ok_or_else(|| miette!("Homebrew is required but could not be found"))?;
            let status = if upgrade && package_installed(runtime, pm, package)? {
                Command::new(&brew)
                    .arg("upgrade")
                    .arg(package)
                    .status()
                    .into_diagnostic()?
            } else {
                Command::new(&brew)
                    .arg("install")
                    .arg(package)
                    .status()
                    .into_diagnostic()?
            };
            if status.success() {
                Ok(())
            } else {
                Err(miette!("brew failed for package {}", package))
            }
        }
    }
}

pub fn pacman_selective_upgrade_supported(pm: PackageManagerId) -> bool {
    pm != PackageManagerId::Pacman
}

fn augmented_path(home: &Path) -> OsString {
    let mut paths = vec![
        home.join(".local/bin"),
        home.join(".cargo/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
    ];

    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }

    std::env::join_paths(paths)
        .unwrap_or_else(|_| OsString::from(std::env::var("PATH").unwrap_or_default()))
}

fn command_failed(program: &str, args: &[&str], stderr: &[u8]) -> miette::Report {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if stderr.is_empty() {
        miette!("command failed: {} {}", program, args.join(" "))
    } else {
        miette!("command failed: {} {}\n{}", program, args.join(" "), stderr)
    }
}

fn is_root() -> Result<bool> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .into_diagnostic()
        .wrap_err("failed to determine effective uid")?;
    if !output.status.success() {
        return Err(miette!("failed to determine effective uid"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim() == "0")
}
