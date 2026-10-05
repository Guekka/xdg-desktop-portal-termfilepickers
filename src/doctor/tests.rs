//! Tests for the backend resolution logic.
//!
//! `XDG_DESKTOP_PORTAL_DIR` makes xdg-desktop-portal ignore every other lookup
//! directory, and we honour it too, so a temporary directory is enough to build
//! a complete fake environment. Since that relies on process-wide environment
//! variables, these tests are serialised behind a mutex.

use std::{
    path::Path,
    sync::{Mutex, MutexGuard, OnceLock},
};

use tempfile::TempDir;

use super::portals::{PortalEnvironment, Resolution, FILE_CHOOSER_IFACE, OUR_DBUS_NAME};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

struct FakeEnv {
    dir: TempDir,
}

impl FakeEnv {
    fn new() -> Self {
        Self {
            dir: TempDir::new().expect("temp dir"),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn portal(self, name: &str, dbus_name: &str, interfaces: &str) -> Self {
        self.write(
            &format!("{name}.portal"),
            &format!("[portal]\nDBusName={dbus_name}\nInterfaces={interfaces};\n"),
        )
    }

    fn portal_with_use_in(self, name: &str, dbus_name: &str, use_in: &str) -> Self {
        self.write(
            &format!("{name}.portal"),
            &format!(
                "[portal]\nDBusName={dbus_name}\nInterfaces={FILE_CHOOSER_IFACE};\nUseIn={use_in};\n"
            ),
        )
    }

    fn conf(self, name: &str, body: &str) -> Self {
        self.write(name, &format!("[preferred]\n{body}"))
    }

    fn write(self, name: &str, content: &str) -> Self {
        std::fs::write(self.dir.path().join(name), content).expect("write");
        self
    }

    /// Discovers the environment with the temp dir in place of the real dirs.
    fn discover(&self, desktops: &str) -> PortalEnvironment {
        let _guard = env_lock();

        let previous_dir = std::env::var_os("XDG_DESKTOP_PORTAL_DIR");
        let previous_desktop = std::env::var_os("XDG_CURRENT_DESKTOP");

        std::env::set_var("XDG_DESKTOP_PORTAL_DIR", self.path());
        std::env::set_var("XDG_CURRENT_DESKTOP", desktops);

        let environment = PortalEnvironment::discover();

        match previous_dir {
            Some(value) => std::env::set_var("XDG_DESKTOP_PORTAL_DIR", value),
            None => std::env::remove_var("XDG_DESKTOP_PORTAL_DIR"),
        }
        match previous_desktop {
            Some(value) => std::env::set_var("XDG_CURRENT_DESKTOP", value),
            None => std::env::remove_var("XDG_CURRENT_DESKTOP"),
        }

        environment
    }
}

#[test]
fn detects_our_backend() {
    let env = FakeEnv::new().portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE);
    let discovered = env.discover("niri");

    let portal = discovered.ours().expect("our backend is found");
    assert_eq!(portal.name, "termfilepickers");
    assert!(portal.supports(FILE_CHOOSER_IFACE));
}

#[test]
fn interface_entry_wins_over_default() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .portal(
            "gtk",
            "org.freedesktop.impl.portal.desktop.gtk",
            FILE_CHOOSER_IFACE,
        )
        .conf(
            "portals.conf",
            &format!("default=gtk\n{FILE_CHOOSER_IFACE}=termfilepickers\n"),
        );

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config {
            name, via_default, ..
        } => {
            assert_eq!(name, "termfilepickers");
            assert!(!via_default);
        }
        other => panic!("unexpected resolution: {other:?}"),
    }
}

/// The situation from the original report: the config selects another backend,
/// so the terminal picker never runs even though everything else is in place.
#[test]
fn reports_another_backend_winning() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .portal(
            "gnome",
            "org.freedesktop.impl.portal.desktop.gnome",
            FILE_CHOOSER_IFACE,
        )
        .conf("portals.conf", "default=gnome\n");

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config {
            name, via_default, ..
        } => {
            assert_eq!(name, "gnome");
            assert!(via_default);
        }
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn desktop_specific_config_wins_in_a_directory() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .portal(
            "gtk",
            "org.freedesktop.impl.portal.desktop.gtk",
            FILE_CHOOSER_IFACE,
        )
        .conf("niri-portals.conf", &format!("{FILE_CHOOSER_IFACE}=gtk\n"))
        .conf(
            "portals.conf",
            &format!("{FILE_CHOOSER_IFACE}=termfilepickers\n"),
        );

    let discovered = env.discover("niri");

    assert_eq!(discovered.configs.len(), 1);
    assert_eq!(discovered.configs[0].desktop.as_deref(), Some("niri"));

    match discovered.resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config { name, .. } => assert_eq!(name, "gtk"),
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn skips_backends_that_do_not_support_the_interface() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .portal(
            "darkman",
            "org.freedesktop.impl.portal.desktop.darkman",
            "org.freedesktop.impl.portal.Settings",
        )
        .conf(
            "portals.conf",
            &format!("{FILE_CHOOSER_IFACE}=darkman;termfilepickers\n"),
        );

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config { name, .. } => assert_eq!(name, "termfilepickers"),
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn detects_none() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .conf("portals.conf", &format!("{FILE_CHOOSER_IFACE}=none\n"));

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::None { .. } => {}
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn detects_use_in_fallback() {
    let env = FakeEnv::new().portal_with_use_in(
        "termfilechooser",
        "org.freedesktop.impl.portal.desktop.termfilechooser",
        "niri",
    );

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::UseIn { name, desktop } => {
            assert_eq!(name, "termfilechooser");
            assert_eq!(desktop, "niri");
        }
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn detects_gtk_last_resort_fallback() {
    let env = FakeEnv::new().portal(
        "gtk",
        "org.freedesktop.impl.portal.desktop.gtk",
        FILE_CHOOSER_IFACE,
    );

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::GtkFallback { name } => assert_eq!(name, "gtk"),
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn detects_nothing_available() {
    let env = FakeEnv::new();

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::Nothing => {}
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn wildcard_resolves_alphabetically() {
    let env = FakeEnv::new()
        .portal("termfilepickers", OUR_DBUS_NAME, FILE_CHOOSER_IFACE)
        .portal(
            "gtk",
            "org.freedesktop.impl.portal.desktop.gtk",
            FILE_CHOOSER_IFACE,
        )
        .conf("portals.conf", &format!("{FILE_CHOOSER_IFACE}=*\n"));

    match env.discover("niri").resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config { name, .. } => assert_eq!(name, "gtk"),
        other => panic!("unexpected resolution: {other:?}"),
    }
}

#[test]
fn invalid_portal_file_is_reported() {
    let env = FakeEnv::new().write(
        "termfilepickers.portal",
        &format!("[portal]\nDBusName={OUR_DBUS_NAME}\n"),
    );

    let discovered = env.discover("niri");
    let portal = discovered.ours().expect("still discovered");

    assert!(portal.error.is_some());
    assert!(!portal.supports(FILE_CHOOSER_IFACE));
}
