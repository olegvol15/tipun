use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

const LATEST_RELEASE: &str = "https://github.com/olegvol15/tipun/releases/latest";
const RAW_FILES: &str = "https://raw.githubusercontent.com/olegvol15/tipun";
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug)]
pub enum UpdateError {
    NoStateDir,
    NotInstalledByScript,
    NoLatestRelease,
    InstallFailed,
    Io(io::Error),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UpdateError::NoStateDir => write!(f, "no state directory (HOME is not set)"),
            UpdateError::NotInstalledByScript => write!(f, "tipun was not installed by install.sh"),
            UpdateError::NoLatestRelease => write!(f, "could not find the latest release"),
            UpdateError::InstallFailed => write!(f, "the install script failed"),
            UpdateError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<io::Error> for UpdateError {
    fn from(err: io::Error) -> Self {
        UpdateError::Io(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    UpToDate,
    Updated(String),
}

pub fn in_background() -> Result<(), UpdateError> {
    if env::var_os("TIPUN_NO_UPDATE").is_some() {
        return Ok(());
    }
    let dir = state_dir().ok_or(UpdateError::NoStateDir)?;
    show_notice(&dir)?;
    let stamp = dir.join("last-update-check");
    let last = fs::metadata(&stamp).and_then(|meta| meta.modified()).ok();
    if !is_due(last, SystemTime::now()) {
        return Ok(());
    }
    fs::create_dir_all(&dir)?;
    fs::write(&stamp, b"")?;
    Command::new(env::current_exe()?)
        .args(["update", "--quiet"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(())
}

pub fn run(quiet: bool) -> Result<Outcome, UpdateError> {
    let dir = state_dir().ok_or(UpdateError::NoStateDir)?;
    let install_dir = installed_by_script(&dir)?;
    let tag = latest_tag()?;
    if !is_newer(&tag, env!("CARGO_PKG_VERSION")) {
        return Ok(Outcome::UpToDate);
    }
    fs::create_dir_all(&dir)?;
    let script = dir.join("install.sh");
    let downloaded = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&script)
        .arg(install_script(&tag))
        .stderr(Stdio::null())
        .status()?;
    if !downloaded.success() {
        return Err(UpdateError::InstallFailed);
    }
    let mut install = Command::new("sh");
    install
        .arg(&script)
        .env("TIPUN_VERSION", &tag)
        .env("TIPUN_INSTALL_DIR", &install_dir)
        .env("TIPUN_STATE_DIR", &dir);
    if quiet {
        install.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let installed = install.status()?;
    fs::remove_file(&script)?;
    if !installed.success() {
        return Err(UpdateError::InstallFailed);
    }
    if quiet {
        fs::write(dir.join("updated"), &tag)?;
    }
    Ok(Outcome::Updated(tag))
}

pub fn state_dir() -> Option<PathBuf> {
    let non_empty = |name| env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) = non_empty("TIPUN_STATE_DIR") {
        return Some(PathBuf::from(dir));
    }
    let base = match non_empty("XDG_STATE_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(non_empty("HOME")?).join(".local/state"),
    };
    Some(base.join("tipun"))
}

fn show_notice(dir: &Path) -> Result<(), UpdateError> {
    let path = dir.join("updated");
    match fs::read_to_string(&path) {
        Ok(tag) => {
            eprintln!("tipun updated to {}", tag.trim());
            fs::remove_file(&path)?;
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn installed_by_script(dir: &Path) -> Result<PathBuf, UpdateError> {
    let recorded = match fs::read_to_string(dir.join("install-path")) {
        Ok(path) => PathBuf::from(path.trim()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(UpdateError::NotInstalledByScript);
        }
        Err(err) => return Err(err.into()),
    };
    let exe = fs::canonicalize(env::current_exe()?)?;
    let install_dir = exe.parent().ok_or(UpdateError::NotInstalledByScript)?;
    match fs::canonicalize(&recorded) {
        Ok(recorded) if recorded == install_dir => Ok(recorded),
        Ok(_) => Err(UpdateError::NotInstalledByScript),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(UpdateError::NotInstalledByScript),
        Err(err) => Err(err.into()),
    }
}

fn latest_tag() -> Result<String, UpdateError> {
    let output = Command::new("curl")
        .args([
            "-fsSLI",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            LATEST_RELEASE,
        ])
        .stderr(Stdio::null())
        .output()?;
    let url = String::from_utf8_lossy(&output.stdout);
    let tag = url.rsplit('/').next().unwrap_or_default();
    if output.status.success() && parse_version(tag).is_some() {
        Ok(tag.to_string())
    } else {
        Err(UpdateError::NoLatestRelease)
    }
}

fn install_script(tag: &str) -> String {
    format!("{RAW_FILES}/{tag}/install.sh")
}

fn is_due(last: Option<SystemTime>, now: SystemTime) -> bool {
    match last {
        Some(last) => now
            .duration_since(last)
            .map_or(true, |age| age >= CHECK_EVERY),
        None => true,
    }
}

fn is_newer(tag: &str, current: &str) -> bool {
    match (parse_version(tag), parse_version(current)) {
        (Some(tag), Some(current)) => tag > current,
        _ => false,
    }
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.strip_prefix('v').unwrap_or(version).split('.');
    let mut next = || parts.next()?.parse::<u64>().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_versions() {
        let cases = [
            ("v0.1.0", Some((0, 1, 0))),
            ("0.1.0", Some((0, 1, 0))),
            ("v1.12.3", Some((1, 12, 3))),
            ("v0.1", None),
            ("v0.1.0.1", None),
            ("v0.1.0-rc.1", None),
            ("v0.x.0", None),
            ("latest", None),
            ("", None),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_version(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn compares_versions() {
        let cases = [
            ("v0.1.1", "0.1.0", true),
            ("v0.2.0", "0.1.9", true),
            ("v1.0.0", "0.9.9", true),
            ("v0.1.10", "0.1.9", true),
            ("v0.1.0", "0.1.0", false),
            ("v0.1.0", "0.1.1", false),
            ("v0.2.0-rc.1", "0.1.0", false),
            ("nonsense", "0.1.0", false),
        ];
        for (tag, current, expected) in cases {
            assert_eq!(is_newer(tag, current), expected, "{tag} vs {current}");
        }
    }

    #[test]
    fn runs_the_installer_from_the_release_tag() {
        assert_eq!(
            install_script("v0.2.0"),
            "https://raw.githubusercontent.com/olegvol15/tipun/v0.2.0/install.sh"
        );
    }

    #[test]
    fn checks_at_most_once_a_day() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * 24 * 60 * 60);
        let hours = |h: u64| Duration::from_secs(h * 60 * 60);
        assert!(is_due(None, now));
        assert!(is_due(Some(now - hours(24)), now));
        assert!(is_due(Some(now - hours(25)), now));
        assert!(!is_due(Some(now - hours(23)), now));
        assert!(!is_due(Some(now), now));
        assert!(is_due(Some(now + hours(1)), now));
    }
}
