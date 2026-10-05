//! Finds out *why* a particular xdg-desktop-portal binary is the one running.
//!
//! A distro portal installed next to a Nix or Home Manager setup is a common
//! combination, and the distro one usually wins: the portal is D-Bus activated
//! and delegates to `xdg-desktop-portal.service`, but systemd reads user units
//! only from `~/.config/systemd/user` and the system directories, never from the
//! Nix profile. The newer binary is installed and simply never started.

use std::path::{Path, PathBuf};

use super::{probe::Probe, xdg};

const UNIT: &str = "xdg-desktop-portal.service";

#[derive(Debug)]
pub struct Newer {
    pub binary: PathBuf,
    pub version: String,
}

#[derive(Debug)]
pub struct Unit {
    pub path: PathBuf,
    /// Provided by the distribution, so shadowing it needs a drop-in override.
    pub is_distro: bool,
}

/// An installed xdg-desktop-portal newer than the running one.
pub fn find_newer_portal(probe: &Probe) -> Option<Newer> {
    let running = probe.version.as_deref().map(parse_version)?;

    candidate_binaries()
        .into_iter()
        .filter(|binary| *binary != probe.binary)
        .filter_map(|binary| {
            let version = read_version(&binary)?;
            Some(Newer { binary, version })
        })
        .filter(|newer| parse_version(&newer.version) > running)
        .max_by_key(|newer| parse_version(&newer.version))
}

/// Places a portal binary may live, derived from the same data dirs the portal
/// itself uses, since `libexec` sits next to `share`.
fn candidate_binaries() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    roots.extend(xdg::data_home().and_then(|dir| dir.parent().map(Path::to_path_buf)));
    roots.extend(
        xdg::data_dirs()
            .into_iter()
            .filter_map(|dir| dir.parent().map(Path::to_path_buf)),
    );

    let mut candidates = Vec::new();
    for root in roots {
        for libexec in ["libexec", "lib/xdg-desktop-portal"] {
            let candidate = root.join(libexec).join("xdg-desktop-portal");
            if xdg::is_executable_file(&candidate) && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

fn read_version(binary: &Path) -> Option<String> {
    let output = std::process::Command::new(binary)
        .arg("--version")
        .output()
        .ok()?;

    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(str::to_owned)
}

fn parse_version(version: &str) -> (u32, u32, u32) {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().unwrap_or(0));

    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// The unit file systemd would use for the portal, found by replicating its
/// search order. Asking systemd directly would be better, but that requires
/// talking to the manager; this only needs to name the file for the hint.
pub fn active_unit() -> Option<Unit> {
    let mut dirs = Vec::new();

    dirs.extend(xdg::config_home().map(|dir| dir.join("systemd/user")));
    dirs.push(PathBuf::from("/etc/systemd/user"));
    dirs.extend(xdg::data_home().map(|dir| dir.join("systemd/user")));
    dirs.push(PathBuf::from("/usr/lib/systemd/user"));
    dirs.push(PathBuf::from("/usr/share/systemd/user"));

    let path = dirs
        .into_iter()
        .map(|dir| dir.join(UNIT))
        .find(|path| path.is_file())?;

    let is_distro = path.starts_with("/usr") || path.starts_with("/etc");

    Some(Unit { path, is_distro })
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn compares_versions_numerically() {
        assert!(parse_version("1.20.4") > parse_version("1.18.4"));
        assert!(parse_version("1.19.0") > parse_version("1.18.99"));
        assert_eq!(parse_version("1.18.4-1ubuntu2"), (1, 18, 4));
    }
}
