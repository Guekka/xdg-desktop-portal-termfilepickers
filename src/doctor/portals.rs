//! Discovery and resolution of xdg-desktop-portal backends.
//!
//! This mirrors the logic of `xdp-portal-config.c` in xdg-desktop-portal so we
//! can tell the user which backend *will* be picked for the FileChooser
//! interface, instead of only checking that our own files are in place.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use super::{keyfile::KeyFile, xdg};

pub const FILE_CHOOSER_IFACE: &str = "org.freedesktop.impl.portal.FileChooser";
pub const OUR_DBUS_NAME: &str = "org.freedesktop.impl.portal.desktop.termfilepickers";
pub const OUR_PORTAL_NAME: &str = "termfilepickers";

/// An installed `*.portal` backend description.
#[derive(Debug, Clone)]
pub struct PortalImpl {
    /// File stem, which is the name used in `portals.conf`.
    pub name: String,
    pub path: PathBuf,
    pub dbus_name: Option<String>,
    pub interfaces: Vec<String>,
    /// The deprecated `UseIn` key.
    pub use_in: Vec<String>,
    /// Set when the file could not be used by xdg-desktop-portal at all.
    pub error: Option<String>,
}

impl PortalImpl {
    fn load(path: &Path) -> Self {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();

        let mut portal = Self {
            name,
            path: path.to_owned(),
            dbus_name: None,
            interfaces: Vec::new(),
            use_in: Vec::new(),
            error: None,
        };

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                portal.error = Some(format!("cannot be read: {err}"));
                return portal;
            }
        };

        let keyfile = KeyFile::parse(&content);

        portal.dbus_name = keyfile.get("portal", "DBusName").map(str::to_owned);
        portal.interfaces = keyfile.get_list("portal", "Interfaces").unwrap_or_default();
        portal.use_in = keyfile.get_list("portal", "UseIn").unwrap_or_default();

        // xdg-desktop-portal rejects the file outright when either key is absent
        if portal.dbus_name.is_none() {
            portal.error = Some("missing the DBusName key".to_owned());
        } else if keyfile.get("portal", "Interfaces").is_none() {
            portal.error = Some("missing the Interfaces key".to_owned());
        }

        portal
    }

    pub fn supports(&self, interface: &str) -> bool {
        self.error.is_none() && self.interfaces.iter().any(|iface| iface == interface)
    }

    pub fn is_ours(&self) -> bool {
        self.dbus_name.as_deref() == Some(OUR_DBUS_NAME)
    }
}

/// A `portals.conf` (or `DESKTOP-portals.conf`) file that xdg-desktop-portal will read.
#[derive(Debug, Clone)]
pub struct PortalConfigFile {
    pub path: PathBuf,
    /// The desktop this file was selected for, if it is desktop-specific.
    pub desktop: Option<String>,
    /// `default=` entry.
    pub default: Vec<String>,
    /// Per-interface entries.
    pub interfaces: BTreeMap<String, Vec<String>>,
}

impl PortalConfigFile {
    fn load(path: &Path, desktop: Option<String>) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        let keyfile = KeyFile::parse(&content);

        let interfaces = keyfile
            .keys("preferred")
            .into_iter()
            .filter(|key| *key != "default")
            .filter_map(|key| {
                let value = keyfile.get_list("preferred", key)?;
                Some((key.to_owned(), value))
            })
            .collect();

        Some(Self {
            path: path.to_owned(),
            desktop,
            default: keyfile.get_list("preferred", "default").unwrap_or_default(),
            interfaces,
        })
    }

    fn preference_for(&self, interface: &str) -> Option<&Vec<String>> {
        self.interfaces.get(interface)
    }
}

/// How the FileChooser backend ends up being chosen.
#[derive(Debug)]
pub enum Resolution {
    /// Selected by an explicit interface entry or by `default=`.
    Config {
        name: String,
        config: PathBuf,
        /// True when it came from `default=` rather than an interface entry.
        via_default: bool,
    },
    /// Explicitly disabled with `none`.
    None { config: PathBuf },
    /// No config matched; picked through the deprecated `UseIn` key.
    UseIn { name: String, desktop: String },
    /// Last-resort fallback to xdg-desktop-portal-gtk.
    GtkFallback { name: String },
    /// Nothing at all could be selected.
    Nothing,
}

#[derive(Debug)]
pub struct PortalEnvironment {
    pub desktops: Vec<String>,
    pub impl_dirs: Vec<PathBuf>,
    pub config_dirs: Vec<PathBuf>,
    /// Every discovered backend, in the order xdg-desktop-portal sorts them.
    pub impls: Vec<PortalImpl>,
    /// Backend files shadowed by a higher-precedence file of the same name.
    pub shadowed: Vec<PortalImpl>,
    /// The config files that apply, highest precedence first.
    pub configs: Vec<PortalConfigFile>,
}

impl PortalEnvironment {
    pub fn discover() -> Self {
        let desktops = xdg::current_desktops();
        let impl_dirs = xdg::portal_impl_dirs();
        let config_dirs = xdg::portal_config_dirs();

        let (impls, shadowed) = discover_impls(&impl_dirs, &desktops);
        let configs = discover_configs(&config_dirs, &desktops);

        Self {
            desktops,
            impl_dirs,
            config_dirs,
            impls,
            shadowed,
            configs,
        }
    }

    pub fn find_impl(&self, name: &str) -> Option<&PortalImpl> {
        self.impls.iter().find(|portal| portal.name == name)
    }

    pub fn ours(&self) -> Option<&PortalImpl> {
        self.impls.iter().find(|portal| portal.is_ours())
    }

    /// Replays the backend selection xdg-desktop-portal performs for `interface`.
    pub fn resolve(&self, interface: &str) -> Resolution {
        for config in &self.configs {
            let explicit = config.preference_for(interface);

            // `none` in the interface entry, or in `default` when there is no
            // interface entry, disables the interface entirely
            let disabling = explicit.unwrap_or(&config.default);
            if disabling.iter().any(|entry| entry == "none") {
                return Resolution::None {
                    config: config.path.clone(),
                };
            }

            for (candidates, via_default) in [(explicit, false), (Some(&config.default), true)] {
                let Some(candidates) = candidates else {
                    continue;
                };

                if let Some(name) = self.pick(candidates, interface) {
                    return Resolution::Config {
                        name,
                        config: config.path.clone(),
                        via_default,
                    };
                }
            }
        }

        for desktop in &self.desktops {
            let found = self.impls.iter().find(|portal| {
                portal.supports(interface)
                    && portal
                        .use_in
                        .iter()
                        .any(|entry| entry.eq_ignore_ascii_case(desktop))
            });

            if let Some(portal) = found {
                return Resolution::UseIn {
                    name: portal.name.clone(),
                    desktop: desktop.clone(),
                };
            }
        }

        let gtk = self.impls.iter().find(|portal| {
            portal.dbus_name.as_deref() == Some("org.freedesktop.impl.portal.desktop.gtk")
                && portal.supports(interface)
        });

        match gtk {
            Some(portal) => Resolution::GtkFallback {
                name: portal.name.clone(),
            },
            None => Resolution::Nothing,
        }
    }

    /// First entry of `candidates` that exists and supports `interface`.
    fn pick(&self, candidates: &[String], interface: &str) -> Option<String> {
        for candidate in candidates {
            if candidate == "none" {
                return None;
            }

            if candidate == "*" {
                return self
                    .impls
                    .iter()
                    .find(|portal| portal.supports(interface))
                    .map(|portal| portal.name.clone());
            }

            if let Some(portal) = self.find_impl(candidate) {
                if portal.supports(interface) {
                    return Some(portal.name.clone());
                }
            }
        }

        None
    }
}

fn discover_impls(dirs: &[PathBuf], desktops: &[String]) -> (Vec<PortalImpl>, Vec<PortalImpl>) {
    let mut found: Vec<PortalImpl> = Vec::new();
    let mut shadowed: Vec<PortalImpl> = Vec::new();

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "portal"))
            .collect();
        paths.sort();

        for path in paths {
            let portal = PortalImpl::load(&path);

            // xdg-desktop-portal keeps the first file found for a given name
            if found.iter().any(|other| other.name == portal.name) {
                shadowed.push(portal);
            } else {
                found.push(portal);
            }
        }
    }

    found.sort_by(|a, b| {
        // backends claiming the current desktop through UseIn come first, then
        // alphabetical order: this decides what `*` resolves to
        let rank = |portal: &PortalImpl| {
            desktops
                .iter()
                .position(|desktop| {
                    portal
                        .use_in
                        .iter()
                        .any(|entry| entry.eq_ignore_ascii_case(desktop))
                })
                .unwrap_or(usize::MAX)
        };

        rank(a).cmp(&rank(b)).then_with(|| a.name.cmp(&b.name))
    });

    (found, shadowed)
}

fn discover_configs(dirs: &[PathBuf], desktops: &[String]) -> Vec<PortalConfigFile> {
    let mut configs = Vec::new();

    for dir in dirs {
        // within a directory, a desktop-specific file wins and stops the search
        let desktop_specific = desktops.iter().find_map(|desktop| {
            let path = dir.join(format!("{desktop}-portals.conf"));
            PortalConfigFile::load(&path, Some(desktop.clone()))
        });

        if let Some(config) = desktop_specific {
            configs.push(config);
            continue;
        }

        if let Some(config) = PortalConfigFile::load(&dir.join("portals.conf"), None) {
            configs.push(config);
        }
    }

    configs
}
