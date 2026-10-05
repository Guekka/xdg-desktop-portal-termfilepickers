//! XDG base directory lookups, mirroring what xdg-desktop-portal itself does.

use std::path::{Path, PathBuf};

/// Build-time `sysconfdir` of xdg-desktop-portal. Usually `/etc`, and we have no
/// way to know if the local build differs.
const SYSCONFDIR: &str = "/etc";
/// Build-time `datadir` of xdg-desktop-portal. Usually `/usr/share`.
const DATADIR: &str = "/usr/share";

const XDP_SUBDIR: &str = "xdg-desktop-portal";

fn env_path(var: &str) -> Option<PathBuf> {
    let value = std::env::var_os(var)?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    // the spec says relative paths must be ignored
    path.is_absolute().then_some(path)
}

fn env_path_list(var: &str, default: &[&str]) -> Vec<PathBuf> {
    let from_env = std::env::var(var).ok().filter(|value| !value.is_empty());

    match from_env {
        Some(value) => value
            .split(':')
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .collect(),
        None => default.iter().map(PathBuf::from).collect(),
    }
}

fn home() -> Option<PathBuf> {
    env_path("HOME")
}

pub fn config_home() -> Option<PathBuf> {
    env_path("XDG_CONFIG_HOME").or_else(|| home().map(|home| home.join(".config")))
}

pub fn data_home() -> Option<PathBuf> {
    env_path("XDG_DATA_HOME").or_else(|| home().map(|home| home.join(".local/share")))
}

pub fn config_dirs() -> Vec<PathBuf> {
    env_path_list("XDG_CONFIG_DIRS", &["/etc/xdg"])
}

pub fn data_dirs() -> Vec<PathBuf> {
    env_path_list("XDG_DATA_DIRS", &["/usr/local/share", "/usr/share"])
}

/// Set by the test suite of xdg-desktop-portal; when present it replaces every
/// other lookup directory.
pub fn portal_dir_override() -> Option<PathBuf> {
    env_path("XDG_DESKTOP_PORTAL_DIR")
}

/// Directories scanned for `*.portal` backend files, highest precedence first.
pub fn portal_impl_dirs() -> Vec<PathBuf> {
    if let Some(dir) = portal_dir_override() {
        return vec![dir];
    }

    let mut dirs = Vec::new();

    dirs.extend(data_home().map(|dir| dir.join(XDP_SUBDIR).join("portals")));
    dirs.extend(
        data_dirs()
            .into_iter()
            .map(|dir| dir.join(XDP_SUBDIR).join("portals")),
    );
    dirs.push(Path::new(DATADIR).join(XDP_SUBDIR).join("portals"));

    dedup(dirs)
}

/// Directories scanned for `portals.conf`, highest precedence first.
pub fn portal_config_dirs() -> Vec<PathBuf> {
    if let Some(dir) = portal_dir_override() {
        return vec![dir];
    }

    let mut dirs = Vec::new();

    dirs.extend(config_home().map(|dir| dir.join(XDP_SUBDIR)));
    dirs.extend(config_dirs().into_iter().map(|dir| dir.join(XDP_SUBDIR)));
    dirs.push(Path::new(SYSCONFDIR).join(XDP_SUBDIR));
    dirs.extend(data_home().map(|dir| dir.join(XDP_SUBDIR)));
    dirs.extend(data_dirs().into_iter().map(|dir| dir.join(XDP_SUBDIR)));
    dirs.push(Path::new(DATADIR).join(XDP_SUBDIR));

    dedup(dirs)
}

/// The entries of `XDG_CURRENT_DESKTOP`, lowercased, as used to pick a
/// desktop-specific `portals.conf`.
pub fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .map(str::trim)
        .filter(|desktop| !desktop.is_empty())
        .filter(|desktop| {
            desktop
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .map(str::to_lowercase)
        .collect()
}

fn dedup(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::with_capacity(dirs.len());
    for dir in dirs {
        if !seen.contains(&dir) {
            seen.push(dir);
        }
    }
    seen
}

/// Resolve an executable the way a shell would, against `path` (a `PATH`-style
/// list). Absolute and relative paths are returned as-is when executable.
pub fn which_in(program: &str, path: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let candidate = PathBuf::from(program);
        return is_executable_file(&candidate).then_some(candidate);
    }

    path.split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(program))
        .find(|candidate| is_executable_file(candidate))
}

pub fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
