//! The individual diagnostics run by `doctor`.

use std::path::{Path, PathBuf};

use zbus::blocking::{fdo::DBusProxy, Connection};

use crate::config::Config;

use super::{
    install,
    portals::{PortalEnvironment, Resolution, FILE_CHOOSER_IFACE, OUR_DBUS_NAME, OUR_PORTAL_NAME},
    probe::{Probe, ProbeError, Selection},
    report::{Check, Status},
    xdg,
};

/// Loading the config can fail, and `doctor` still has plenty to report in that
/// case, so the outcome is kept alongside the checks.
pub struct ConfigOutcome {
    pub config: Option<Config>,
    pub checks: Vec<Check>,
}

pub fn check_config(path: Option<&Path>, load_error: Option<&anyhow::Error>) -> ConfigOutcome {
    let mut checks = Vec::new();

    let Some(path) = path else {
        checks.push(
            Check::fail("config file", "not found")
                .with_note("looked for config.toml in the termfilepickers XDG config dirs")
                .with_hint("pass --config-path, or let the Nix module generate the config")
                .with_hint(
                    "the systemd unit from the Nix module passes --config-path to the binary",
                ),
        );

        return ConfigOutcome {
            config: None,
            checks,
        };
    };

    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            checks.push(
                Check::fail("config file", format!("{} cannot be read", path.display()))
                    .with_note(err.to_string()),
            );
            return ConfigOutcome {
                config: None,
                checks,
            };
        }
    };

    let config: Config = match toml::from_str(&content) {
        Ok(config) => config,
        Err(err) => {
            checks.push(
                Check::fail("config file", format!("{} is not valid", path.display()))
                    .with_notes(err.to_string().lines().map(str::to_owned)),
            );
            return ConfigOutcome {
                config: None,
                checks,
            };
        }
    };

    checks.push(Check::ok("config file", path.display().to_string()));

    if let Some(err) = load_error {
        tracing::debug!("config was rejected at startup: {err:?}");
    }

    for (option, script) in config.scripts() {
        checks.push(check_script(option, script));
    }

    checks.push(check_terminal_command(&config));
    checks.extend(check_script_interpreters(&config));

    ConfigOutcome {
        config: Some(config),
        checks,
    }
}

fn check_script(option: &str, script: &Path) -> Check {
    match Config::is_valid_script(script) {
        Ok(()) => Check::ok(option, script.display().to_string()),
        Err(err) => Check::fail(option, script.display().to_string())
            .with_note(err.to_string())
            .with_hint("the path must point to an existing executable file"),
    }
}

fn check_terminal_command(config: &Config) -> Check {
    let Some(program) = config.terminal_command.first() else {
        return Check::fail("terminal_command", "empty")
            .with_hint("set it to your terminal, for example [(lib.getExe pkgs.kitty)]");
    };

    let rendered = config.terminal_command.join(" ");

    let path = std::env::var("PATH").unwrap_or_default();
    let Some(resolved) = xdg::which_in(program, &path) else {
        return Check::fail("terminal_command", rendered)
            .with_note(format!("{program} was not found"))
            .with_hint("use an absolute path, since the service may not inherit your PATH")
            .with_hint("with the Nix module: terminal_command = [(lib.getExe pkgs.kitty)]");
    };

    let mut check = Check::ok("terminal_command", rendered)
        .with_note(format!("{program} resolves to {}", resolved.display()));

    // the terminal must be told to treat the rest of the arguments as a command
    // to run; most terminals need an explicit flag for that
    let needs_exec_flag = ["ghostty", "gnome-terminal", "xfce4-terminal", "konsole"];
    let binary = Path::new(program)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let has_flag = config
        .terminal_command
        .iter()
        .skip(1)
        .any(|arg| arg.starts_with('-'));

    if needs_exec_flag.iter().any(|name| binary.contains(name)) && !has_flag {
        check.status = Status::Warn;
        check.hints.push(format!(
            "{binary} usually needs a flag before the command to run (ghostty: -e, gnome-terminal: --, konsole: -e)"
        ));
    }

    check
}

/// The bundled wrappers are nushell scripts calling yazi, and a missing `PATH`
/// in the service environment is a common cause of silent failures.
fn check_script_interpreters(config: &Config) -> Vec<Check> {
    let path = std::env::var("PATH").unwrap_or_default();
    let mut programs: Vec<String> = Vec::new();

    for (_, script) in config.scripts() {
        let Ok(content) = std::fs::read_to_string(script) else {
            continue;
        };

        if let Some(shebang) = content.lines().next().and_then(|l| l.strip_prefix("#!")) {
            let mut words = shebang.split_whitespace();
            let first = words.next().unwrap_or_default();
            // "#!/usr/bin/env nu" -> nu
            let program = if first.ends_with("/env") {
                words.next().unwrap_or_default()
            } else {
                first
            };

            if !program.is_empty() {
                programs.push(program.to_owned());
            }
        }

        // the wrappers either call "yazi" from PATH or a patched absolute path
        if content.contains("yazi") {
            let quoted = content
                .split('"')
                .find(|part| part.ends_with("yazi"))
                .unwrap_or("yazi");
            programs.push(quoted.to_owned());
        }
    }

    programs.sort();
    programs.dedup();

    programs
        .into_iter()
        .map(|program| {
            let name = format!("script dependency '{program}'");
            match xdg::which_in(&program, &path) {
                Some(resolved) => Check::ok(name, resolved.display().to_string()),
                None => Check::fail(name, "not found")
                    .with_note("referenced by one of the configured scripts")
                    .with_hint(
                        "install it, or use a package built with the Nix flake so the wrappers \
                         embed absolute paths",
                    )
                    .with_hint(
                        "the service does not necessarily inherit your interactive shell PATH",
                    ),
            }
        })
        .collect()
}

pub fn check_session(env: &PortalEnvironment) -> Vec<Check> {
    let mut checks = Vec::new();

    if env.desktops.is_empty() {
        checks.push(
            Check::warn("XDG_CURRENT_DESKTOP", "not set")
                .with_note("only the non desktop-specific portals.conf will be read")
                .with_hint(
                    "if your compositor does not set it, put your preferences in portals.conf",
                ),
        );
    } else {
        checks.push(
            Check::ok("XDG_CURRENT_DESKTOP", env.desktops.join(":")).with_note(format!(
                "config lookup order: {}",
                env.desktops
                    .iter()
                    .map(|desktop| format!("{desktop}-portals.conf"))
                    .chain(std::iter::once("portals.conf".to_owned()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        );
    }

    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let x11 = std::env::var_os("DISPLAY").is_some();

    let detail = match (wayland, x11) {
        (true, _) => "wayland".to_owned(),
        (false, true) => "x11".to_owned(),
        (false, false) => "no display server detected".to_owned(),
    };

    if wayland || x11 {
        let mut check = Check::ok("graphical session", detail);
        if !session_type.is_empty() {
            check = check.with_note(format!("XDG_SESSION_TYPE={session_type}"));
        }
        checks.push(check);
    } else {
        checks.push(
            Check::warn("graphical session", detail)
                .with_note("neither WAYLAND_DISPLAY nor DISPLAY is set")
                .with_hint(
                    "the Nix module gates the service on WAYLAND_DISPLAY, so it may not start",
                )
                .with_hint("if you are in a TTY or over SSH, run doctor from your desktop session"),
        );
    }

    if xdg::portal_dir_override().is_some() {
        checks.push(
            Check::warn(
                "XDG_DESKTOP_PORTAL_DIR",
                "set, overriding every portal lookup directory",
            )
            .with_hint("unset it unless you are running the xdg-desktop-portal test suite"),
        );
    }

    checks
}

pub fn check_portal_file(env: &PortalEnvironment) -> Vec<Check> {
    let mut checks = Vec::new();

    let dirs = env
        .impl_dirs
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    match env.ours() {
        Some(portal) => {
            let mut check = Check::ok("termfilepickers.portal", portal.path.display().to_string());

            if let Some(error) = &portal.error {
                check.status = Status::Fail;
                check.detail = format!("{} is invalid", portal.path.display());
                check = check
                    .with_note(error.clone())
                    .with_hint("reinstall the package: the .portal file is provided by it");
            } else if !portal.supports(FILE_CHOOSER_IFACE) {
                check.status = Status::Fail;
                check = check
                    .with_note(format!("does not declare {FILE_CHOOSER_IFACE}"))
                    .with_hint("reinstall the package: the .portal file is provided by it");
            } else if portal.name != OUR_PORTAL_NAME {
                check.status = Status::Warn;
                check = check
                    .with_note(format!(
                        "installed under the name '{}', not '{OUR_PORTAL_NAME}'",
                        portal.name
                    ))
                    .with_hint(format!(
                        "portals.conf must refer to it as '{}'",
                        portal.name
                    ));
            }

            checks.push(check);
        }
        None => checks.push(
            Check::fail("termfilepickers.portal", "not installed")
                .with_note(format!("searched: {dirs}"))
                .with_hint("add the package to xdg.portal.extraPortals")
                .with_hint("the Nix module does this for you when xdg.portal.enable = true")
                .with_hint(
                    "the file must be visible in XDG_DATA_DIRS of the xdg-desktop-portal process",
                ),
        ),
    }

    let others: Vec<String> = env
        .impls
        .iter()
        .filter(|portal| !portal.is_ours() && portal.supports(FILE_CHOOSER_IFACE))
        .map(|portal| portal.name.clone())
        .collect();

    if !others.is_empty() {
        checks.push(
            Check::ok("competing FileChooser backends", others.join(", ")).with_note(
                "these also implement FileChooser, so the config decides which one wins",
            ),
        );
    }

    // having the same backend installed both system-wide and per-user is normal,
    // so this is context rather than a problem
    for portal in env.shadowed.iter().filter(|portal| portal.is_ours()) {
        checks.push(
            Check::ok(
                format!("shadowed copy of '{}'", portal.name),
                portal.path.display().to_string(),
            )
            .with_note(
                "ignored: a file with the same name was found in a higher-precedence directory",
            ),
        );
    }

    checks
}

/// What the running xdg-desktop-portal reports about itself.
///
/// This is authoritative where our own model cannot be: only the running binary
/// knows its build-time directories, and several versions may be installed at
/// once. Versions before 1.19 ignore `XDG_DATA_DIRS` when looking for `*.portal`
/// files, so a correctly installed backend can be invisible to them.
pub fn check_probe(probe: &Result<Probe, ProbeError>, env: &PortalEnvironment) -> Vec<Check> {
    let probe = match probe {
        Ok(probe) => probe,
        Err(err) => {
            return vec![Check::warn("running xdg-desktop-portal", err.to_string())
                .with_note("falling back to our own model of the backend selection")
                .with_note(
                    "that model is a reimplementation and may not match your version, so the \
                     verdict below is a guess",
                )
                .with_hint("run doctor from inside your desktop session for a definitive answer")
                .with_hint("or compare against: xdg-desktop-portal -vr")];
        }
    };

    let mut checks = Vec::new();

    let version = probe.version.as_deref().unwrap_or("unknown version");
    checks.push(
        Check::ok(
            "running xdg-desktop-portal",
            format!("{} ({version})", probe.binary.display()),
        )
        .with_note("this is the binary that currently owns the portal bus name"),
    );

    checks.push(check_backend_visibility(probe, env));
    checks.push(check_probe_selection(probe, env));

    checks
}

/// Our `.portal` file existing is not enough: the running binary has to look in
/// the directory that contains it.
fn check_backend_visibility(probe: &Probe, env: &PortalEnvironment) -> Check {
    let name = "backend visible to xdg-desktop-portal";

    let Some(ours) = env.ours() else {
        return Check::warn(name, "skipped, no backend file found")
            .with_note("see the Portal backend section");
    };

    let seen = probe
        .loaded
        .iter()
        .any(|path| path.file_name() == ours.path.file_name());

    if seen {
        return Check::ok(name, "yes");
    }

    let scanned = probe
        .scanned_dirs
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut check = Check::fail(name, "no")
        .with_note(format!(
            "our backend is installed at {}",
            ours.path.display()
        ))
        .with_note(format!("but xdg-desktop-portal only scanned: {scanned}"));

    // the version cutoff explains most of these, and the fix differs
    if probe.version.as_deref().is_some_and(is_before_1_19) {
        check = check
            .with_note(format!(
                "xdg-desktop-portal {} only looks in its build-time directory and ignores XDG_DATA_DIRS",
                probe.version.as_deref().unwrap_or_default()
            ))
            .with_note("support for XDG_DATA_DIRS was added in 1.19");

        check = match install::find_newer_portal(probe) {
            Some(newer) => explain_install_mismatch(check, &newer),
            None => check
                .with_hint("upgrade xdg-desktop-portal to 1.19 or later")
                .with_hint(format!(
                    "or install this package system-wide, into {}",
                    scanned_hint(probe)
                )),
        };
    } else {
        check = check
            .with_hint("install the package so its .portal file lands in a scanned directory")
            .with_hint("XDG_DATA_DIRS of the xdg-desktop-portal process must contain it");
    }

    check
}

/// A newer xdg-desktop-portal is installed but not the one running. This is the
/// usual shape on a distro that ships an old portal next to a Nix/Home Manager
/// setup: systemd activates the distro unit, so the newer binary never runs.
fn explain_install_mismatch(check: Check, newer: &install::Newer) -> Check {
    let mut check = check.with_note(format!(
        "a newer xdg-desktop-portal {} is installed at {}, but it is not the one running",
        newer.version,
        newer.binary.display()
    ));

    let Some(unit) = install::active_unit() else {
        return check
            .with_hint(format!("make {} the one that runs", newer.binary.display()))
            .with_hint("the portal is D-Bus activated, so the service definition decides");
    };

    check = check.with_note(format!(
        "the running binary comes from {}",
        unit.path.display()
    ));

    if unit.is_distro {
        check = check
            .with_note(
                "systemd only reads user units from ~/.config/systemd/user and the system \
                 directories, not from the Nix profile, so the distro unit wins",
            )
            .with_hint("upgrading the distribution xdg-desktop-portal past 1.19 is the cleanest fix")
            .with_hint(format!(
                "otherwise a unit in ~/.config/systemd/user overrides it, with ExecStart={}",
                newer.binary.display()
            ))
            .with_hint(
                "that replaces a distribution service and opts out of its updates, so check it first",
            );
    } else {
        check = check.with_hint(format!(
            "point {} at {}",
            unit.path.display(),
            newer.binary.display()
        ));
    }

    check
}

fn scanned_hint(probe: &Probe) -> String {
    probe
        .scanned_dirs
        .first()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|| "the directory it scans".to_owned())
}

/// xdg-desktop-portal's own verdict, plus a comparison with ours.
fn check_probe_selection(probe: &Probe, env: &PortalEnvironment) -> Check {
    let name = "FileChooser backend (reported by xdg-desktop-portal)";
    let ours_is = |backend: &str| {
        backend == OUR_PORTAL_NAME || env.find_impl(backend).is_some_and(|p| p.is_ours())
    };

    let mut check = match &probe.selection {
        Selection::Backend {
            name: backend,
            reason,
        } if ours_is(backend) => {
            Check::ok(name, backend.clone()).with_note(format!("selected via {reason}"))
        }
        Selection::Backend {
            name: backend,
            reason,
        } => Check::fail(name, backend.clone())
            .with_note(format!("selected via {reason}"))
            .with_note(format!(
                "this is why you get the '{backend}' file picker instead of a terminal one"
            ))
            .with_hint(portals_conf_hint()),
        Selection::UseIn { name: backend } => {
            let status = if ours_is(backend) {
                Status::Warn
            } else {
                Status::Fail
            };
            Check::new(name, status, backend.clone())
                .with_note("selected through the deprecated UseIn key")
                .with_hint(portals_conf_hint())
        }
        Selection::LastResort { name: backend } => {
            let mut check = Check::fail(name, backend.clone())
                .with_note("a last-resort fallback: nothing actually selected a backend");

            // when the config does name us, the config is not the problem: the
            // backend was simply not visible to this binary
            if env.configs.iter().any(|config| {
                config
                    .interfaces
                    .get(FILE_CHOOSER_IFACE)
                    .is_some_and(|portals| portals.iter().any(|p| ours_is(p)))
            }) {
                check = check
                    .with_note("your config does request termfilepickers, so the config is fine")
                    .with_hint("see the backend visibility check above");
            } else {
                check = check.with_hint(portals_conf_hint());
            }

            check
        }
        Selection::None => Check::fail(name, "none")
            .with_note("no backend was selected for the FileChooser interface")
            .with_hint(portals_conf_hint()),
    };

    // a disagreement means our model does not match this version's behaviour, and
    // the running binary is the one that decides
    let ours = match env.resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config { name, .. } => Some(name),
        Resolution::UseIn { name, .. } => Some(name),
        Resolution::GtkFallback { name } => Some(name),
        Resolution::None { .. } | Resolution::Nothing => None,
    };

    if ours.as_deref() != probe.selection.name() {
        let expected = ours.as_deref().unwrap_or("none");
        let actual = probe.selection.name().unwrap_or("none");

        check = check
            .with_note(format!(
                "our own analysis expected '{expected}', xdg-desktop-portal reports '{actual}'"
            ))
            .with_note(
                "the reported value comes from xdg-desktop-portal itself, so prefer it over \
                 our analysis",
            );

        if check.status == Status::Ok {
            check.status = Status::Warn;
        }
    }

    check
}

fn is_before_1_19(version: &str) -> bool {
    let mut parts = version.split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);

    (major, minor) < (1, 19)
}

/// `probed` suppresses our own verdict: when xdg-desktop-portal answered for
/// itself, reporting a second, possibly contradictory verdict only confuses.
pub fn check_portal_config(env: &PortalEnvironment, probed: bool) -> Vec<Check> {
    let mut checks = Vec::new();

    if env.configs.is_empty() {
        checks.push(
            Check::fail("portals.conf", "no configuration file found")
                .with_note(format!(
                    "searched: {}",
                    env.config_dirs
                        .iter()
                        .map(|dir| dir.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .with_hint(portals_conf_hint())
                .with_hint("with the Nix module: set desktopEnvironments to your compositor"),
        );
    } else {
        for config in &env.configs {
            let scope = match &config.desktop {
                Some(desktop) => format!("for desktop '{desktop}'"),
                None => "for any desktop".to_owned(),
            };

            let preference = config
                .interfaces
                .get(FILE_CHOOSER_IFACE)
                .map(|portals| portals.join(";"));

            let mut check = Check::ok(
                format!("{} ({scope})", config.path.display()),
                match &preference {
                    Some(value) => format!("FileChooser={value}"),
                    None if config.default.is_empty() => "no relevant entry".to_owned(),
                    None => format!("default={}", config.default.join(";")),
                },
            );

            // only the highest-precedence file that resolves something matters,
            // so a lower-precedence file mentioning us is not a problem per se
            if preference.is_none() && !config.default.is_empty() {
                check = check.with_note("FileChooser falls back to the default entry here");
            }

            checks.push(check);
        }
    }

    if !probed {
        checks.push(check_resolution(env));
    }

    checks
}

/// Only used when the running portal could not be asked. This replays the
/// selection rules of one upstream version and may be wrong on others.
fn check_resolution(env: &PortalEnvironment) -> Check {
    let name = "selected FileChooser backend (our analysis, unverified)";

    match env.resolve(FILE_CHOOSER_IFACE) {
        Resolution::Config {
            name: selected,
            config,
            via_default,
        } if selected == OUR_PORTAL_NAME
            || env.find_impl(&selected).is_some_and(|p| p.is_ours()) =>
        {
            let mut check =
                Check::ok(name, selected).with_note(format!("chosen by {}", config.display()));
            if via_default {
                check = check.with_note("matched through the default entry");
            }
            check
        }
        Resolution::Config {
            name: selected,
            config,
            ..
        } => Check::fail(name, selected.clone())
            .with_note(format!("chosen by {}", config.display()))
            .with_note(format!(
                "this is why you get the '{selected}' file picker instead of a terminal one"
            ))
            .with_hint(format!(
                "in {}, set {FILE_CHOOSER_IFACE}={OUR_PORTAL_NAME}",
                config.display()
            ))
            .with_hint("a higher-precedence config file can also override this"),
        Resolution::None { config } => Check::fail(name, "disabled with 'none'")
            .with_note(format!("set in {}", config.display()))
            .with_hint(format!(
                "replace 'none' with '{OUR_PORTAL_NAME}' for {FILE_CHOOSER_IFACE}"
            )),
        Resolution::UseIn {
            name: selected,
            desktop,
        } => {
            let ours = env.find_impl(&selected).is_some_and(|p| p.is_ours());
            let status = if ours { Status::Warn } else { Status::Fail };

            Check::new(name, status, selected.clone())
                .with_note(format!(
                    "chosen through the deprecated UseIn key, for desktop '{desktop}'"
                ))
                .with_note("no portals.conf entry matched, so this is a fallback")
                .with_hint(portals_conf_hint())
        }
        Resolution::GtkFallback { name: selected } => Check::fail(name, selected)
            .with_note("last-resort fallback because nothing selected a backend")
            .with_hint(portals_conf_hint()),
        Resolution::Nothing => Check::fail(name, "none")
            .with_note("xdg-desktop-portal would not find any FileChooser backend")
            .with_hint(portals_conf_hint()),
    }
}

fn portals_conf_hint() -> String {
    let dir = xdg::config_home()
        .map(|dir| dir.join("xdg-desktop-portal"))
        .unwrap_or_else(|| PathBuf::from("~/.config/xdg-desktop-portal"));

    let file = match xdg::current_desktops().first() {
        Some(desktop) => format!("{desktop}-portals.conf"),
        None => "portals.conf".to_owned(),
    };

    format!(
        "write [preferred] with {FILE_CHOOSER_IFACE}={OUR_PORTAL_NAME} in {}/{file}",
        dir.display()
    )
}

pub fn check_dbus() -> Vec<Check> {
    let mut checks = Vec::new();

    let connection = match Connection::session() {
        Ok(connection) => connection,
        Err(err) => {
            checks.push(
                Check::fail("session bus", "cannot connect")
                    .with_note(err.to_string())
                    .with_hint("doctor must run inside your desktop session"),
            );
            return checks;
        }
    };

    let proxy = match DBusProxy::new(&connection) {
        Ok(proxy) => proxy,
        Err(err) => {
            checks.push(Check::fail("session bus", "cannot be queried").with_note(err.to_string()));
            return checks;
        }
    };

    let names = match proxy.list_names() {
        Ok(names) => names,
        Err(err) => {
            checks.push(Check::fail("session bus", "cannot list names").with_note(err.to_string()));
            return checks;
        }
    };

    let has = |name: &str| names.iter().any(|owned| owned.as_str() == name);

    checks.push(if has("org.freedesktop.portal.Desktop") {
        Check::ok("xdg-desktop-portal", "running")
    } else {
        Check::fail("xdg-desktop-portal", "not running")
            .with_note("org.freedesktop.portal.Desktop is not on the session bus")
            .with_hint("install xdg-desktop-portal and set xdg.portal.enable = true")
            .with_hint("check: systemctl --user status xdg-desktop-portal")
    });

    checks.push(if has(OUR_DBUS_NAME) {
        Check::ok("termfilepickers service", "running")
    } else {
        Check::fail("termfilepickers service", "not running")
            .with_note(format!("{OUR_DBUS_NAME} is not on the session bus"))
            .with_hint("start it: systemctl --user start xdg-desktop-portal-termfilepickers")
            .with_hint("check its logs: journalctl --user -eu xdg-desktop-portal-termfilepickers")
    });

    checks
}

/// Toolkit-side settings that decide whether an application talks to the portal
/// at all. A wrong value here makes the portal look broken even when it is fine.
pub fn check_toolkit_integration() -> Vec<Check> {
    let mut checks = Vec::new();

    let gtk_use_portal = std::env::var("GTK_USE_PORTAL").unwrap_or_default();
    let gdk_debug = std::env::var("GDK_DEBUG").unwrap_or_default();
    let gdk_portals = gdk_debug.split(',').any(|flag| flag.trim() == "portals");

    let mut check = match (gtk_use_portal.as_str(), gdk_portals) {
        ("1", true) => Check::ok("GTK portal usage", "GTK_USE_PORTAL=1, GDK_DEBUG=portals"),
        ("1", false) => Check::warn("GTK portal usage", "GTK_USE_PORTAL=1, GDK_DEBUG unset")
            .with_note("GTK 4 ignores GTK_USE_PORTAL and reads GDK_DEBUG=portals instead"),
        ("", true) => Check::warn(
            "GTK portal usage",
            "GDK_DEBUG=portals, GTK_USE_PORTAL unset",
        )
        .with_note("GTK 3 applications still need GTK_USE_PORTAL=1"),
        _ => Check::warn("GTK portal usage", "not configured")
            .with_note("GTK applications will use their built-in file chooser"),
    };

    if check.status != Status::Ok {
        check = check
            .with_hint("set both GTK_USE_PORTAL=1 and GDK_DEBUG=portals in your session")
            .with_hint("with Home Manager: home.sessionVariables, then log out and back in")
            .with_hint(
                "Firefox-based browsers need widget.use-xdg-desktop-portal.file-picker=1 in \
                 about:config instead",
            );
    }

    checks.push(check);

    checks
}

/// Actually runs the configured open-file script, which is the only way to catch
/// a wrapper that fails at runtime.
pub fn check_script_run(config: &Config) -> Vec<Check> {
    use crate::runner::{ConfigRunner, Runner, RunnerOpenFileOptions};

    let runner = ConfigRunner::new(Config {
        open_file_script_path: config.open_file_script_path.clone(),
        save_file_script_path: config.save_file_script_path.clone(),
        save_files_script_path: config.save_files_script_path.clone(),
        terminal_command: config.terminal_command.clone(),
    });

    let result = runner.run_open_file(&RunnerOpenFileOptions {
        multiple: false,
        directory: false,
    });

    let check = match result {
        // quitting the picker without choosing is a normal outcome, and it still
        // proves the script ran and the terminal worked
        Ok(result) if result.uris.is_empty() => {
            Check::ok("open_file script run", "ran, no file selected")
                .with_note("the picker exited without a selection, which is what cancelling does")
                .with_note("re-run and pick a file to exercise the full path")
        }
        Ok(result) => Check::ok("open_file script run", "succeeded")
            .with_notes(result.uris.iter().map(|uri| format!("returned {uri}"))),
        Err(err) => Check::fail("open_file script run", "failed")
            .with_notes(format!("{err:?}").lines().map(str::to_owned))
            .with_hint("check that the terminal command keeps running until the picker exits")
            .with_hint("check that the file manager used by the wrapper is installed"),
    };

    vec![check]
}
