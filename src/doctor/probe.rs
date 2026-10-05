//! Asks the running xdg-desktop-portal which backend it selects, instead of
//! relying only on our own model of its selection algorithm.
//!
//! Running a second instance with `-v` makes it print its verdict and exit
//! without disturbing the live one: it resolves backends while registering, then
//! loses the bus name and terminates. The output is `g_debug` text with no
//! stability guarantee, which is why this only *complements*
//! [`super::portals`]: the strings have changed in every recent release, while
//! the algorithm has been comparatively stable.

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use zbus::blocking::{fdo::DBusProxy, Connection};

use super::portals::FILE_CHOOSER_IFACE;

const XDP_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

/// What the running xdg-desktop-portal reported for an interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// A backend was selected. `reason` is the wording the binary used.
    Backend { name: String, reason: String },
    /// Selected through the deprecated `UseIn` key.
    UseIn { name: String },
    /// Last-resort fallback, meaning nothing actually selected a backend.
    LastResort { name: String },
    /// The binary mentioned the interface but selected nothing for it.
    None,
}

impl Selection {
    pub fn name(&self) -> Option<&str> {
        match self {
            Selection::Backend { name, .. }
            | Selection::UseIn { name }
            | Selection::LastResort { name } => Some(name),
            Selection::None => None,
        }
    }
}

#[derive(Debug)]
pub struct Probe {
    /// The binary that currently owns the portal bus name.
    pub binary: PathBuf,
    pub version: Option<String>,
    /// Directories it reported scanning for `*.portal` files.
    pub scanned_dirs: Vec<PathBuf>,
    /// `*.portal` files it reported loading.
    pub loaded: Vec<PathBuf>,
    pub selection: Selection,
}

/// Why probing was not possible. Never fatal: doctor falls back to its own model.
#[derive(Debug)]
pub enum ProbeError {
    NotRunning,
    BinaryUnknown(String),
    Failed(String),
    /// Ran, but printed nothing we could interpret.
    Unparsable,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::NotRunning => write!(f, "xdg-desktop-portal is not running"),
            ProbeError::BinaryUnknown(err) => {
                write!(f, "the running binary could not be identified: {err}")
            }
            ProbeError::Failed(err) => write!(f, "could not be run: {err}"),
            ProbeError::Unparsable => write!(f, "produced no recognisable output"),
        }
    }
}

pub fn probe() -> Result<Probe, ProbeError> {
    let binary = running_binary()?;
    let version = binary_version(&binary);

    let text = run_with_timeout(&binary)?;

    parse(&binary, version, &text).ok_or(ProbeError::Unparsable)
}

/// The binary owning the portal bus name right now. Several versions of
/// xdg-desktop-portal can be installed at once, and only this one is relevant.
fn running_binary() -> Result<PathBuf, ProbeError> {
    let connection =
        Connection::session().map_err(|err| ProbeError::BinaryUnknown(err.to_string()))?;
    let proxy =
        DBusProxy::new(&connection).map_err(|err| ProbeError::BinaryUnknown(err.to_string()))?;

    let name = XDP_BUS_NAME
        .try_into()
        .map_err(|_| ProbeError::BinaryUnknown("invalid bus name".to_owned()))?;

    let pid = proxy
        .get_connection_unix_process_id(name)
        .map_err(|_| ProbeError::NotRunning)?;

    let exe = PathBuf::from(format!("/proc/{pid}/exe"));
    std::fs::read_link(&exe)
        .map_err(|err| ProbeError::BinaryUnknown(format!("cannot read {}: {err}", exe.display())))
}

/// The probe is expected to exit on its own once it loses the bus name, but
/// doctor must not hang if it does not.
fn run_with_timeout(binary: &Path) -> Result<String, ProbeError> {
    use std::{io::Read, sync::mpsc, thread, time::Instant};

    let mut child = Command::new(binary)
        .arg("-v")
        .env("G_MESSAGES_DEBUG", "all")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| ProbeError::Failed(err.to_string()))?;

    // both pipes must be drained concurrently, or a full one deadlocks the child
    let (sender, receiver) = mpsc::channel();
    for stream in [
        child.stdout.take().map(Readable::Out),
        child.stderr.take().map(Readable::Err),
    ]
    .into_iter()
    .flatten()
    {
        let sender = sender.clone();
        thread::spawn(move || {
            let mut buffer = String::new();
            let mut stream = stream;
            let _ = stream.read_to_string(&mut buffer);
            let _ = sender.send(buffer);
        });
    }
    drop(sender);

    let deadline = Instant::now() + PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(err) => return Err(ProbeError::Failed(err.to_string())),
        }
    }

    let _ = child.wait();

    Ok(receiver.iter().collect::<Vec<_>>().join(""))
}

enum Readable {
    Out(std::process::ChildStdout),
    Err(std::process::ChildStderr),
}

impl std::io::Read for Readable {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Readable::Out(stream) => stream.read(buf),
            Readable::Err(stream) => stream.read(buf),
        }
    }
}

fn binary_version(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    text.split_whitespace()
        .find(|word| word.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(str::to_owned)
}

fn parse(binary: &Path, version: Option<String>, text: &str) -> Option<Probe> {
    let mut scanned_dirs = Vec::new();
    let mut loaded = Vec::new();
    let mut selection = None;
    let mut saw_interface = false;

    for line in text.lines() {
        // strip the "XDP: " or glib "(binary:pid): domain-LEVEL **: time: " prefix
        let line = line
            .rsplit_once("**: ")
            .map(|(_, rest)| {
                rest.trim_start_matches(|c: char| {
                    c.is_ascii_digit() || c == ':' || c == '.' || c == ' '
                })
            })
            .unwrap_or(line);
        let line = line.trim().trim_start_matches("XDP: ").trim();

        if let Some(dir) = line.strip_prefix("load portals from ") {
            scanned_dirs.push(PathBuf::from(dir.trim()));
            continue;
        }

        if let Some(path) = line.strip_prefix("loading ") {
            let path = path.trim();
            if path.ends_with(".portal") {
                loaded.push(PathBuf::from(path));
            }
            continue;
        }

        if !line.contains(FILE_CHOOSER_IFACE) {
            continue;
        }

        saw_interface = true;

        // "Using <name>.portal for <iface> (config)", and its 1.22 variants
        // "(interface specific config)" / "(default config)", plus the 1.16
        // spelling without the ".portal" suffix
        if let Some(rest) = line.strip_prefix("Using ") {
            if let Some((name, reason)) = parse_using(rest) {
                selection = Some(Selection::Backend { name, reason });
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("Choosing ") {
            let name = backend_name(rest)?;

            if line.contains("last-resort") {
                selection = Some(Selection::LastResort { name });
            } else if line.contains("UseIn") {
                selection = Some(Selection::UseIn { name });
            }
            continue;
        }

        // 1.16/1.18 wording for the wildcard fallback
        if let Some(rest) = line.strip_prefix("Falling back to ") {
            let name = backend_name(rest)?;
            selection = Some(Selection::Backend {
                name,
                reason: "fallback".to_owned(),
            });
        }
    }

    if !saw_interface && scanned_dirs.is_empty() {
        return None;
    }

    Some(Probe {
        binary: binary.to_owned(),
        version,
        scanned_dirs,
        loaded,
        selection: selection.unwrap_or(Selection::None),
    })
}

/// `<name>.portal for <iface> (reason)` -> (name, reason)
fn parse_using(rest: &str) -> Option<(String, String)> {
    let name = backend_name(rest)?;

    let reason = rest
        .rsplit_once('(')
        .and_then(|(_, tail)| tail.strip_suffix(')'))
        .map(str::to_owned)
        // "Using x.portal for y in <desktop> (fallback)" has the reason first
        .unwrap_or_else(|| "config".to_owned());

    Some((name, reason))
}

fn backend_name(rest: &str) -> Option<String> {
    let word = rest.split_whitespace().next()?;
    Some(word.trim_end_matches(".portal").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Probe {
        parse(Path::new("/usr/libexec/xdg-desktop-portal"), None, text).expect("parsed")
    }

    #[test]
    fn parses_1_18_config_selection() {
        let probe = parsed(&format!(
            "XDP: load portals from /usr/share/xdg-desktop-portal/portals\n\
             XDP: loading /usr/share/xdg-desktop-portal/portals/gtk.portal\n\
             XDP: Using termfilepickers.portal for {FILE_CHOOSER_IFACE} (config)\n"
        ));

        assert_eq!(probe.selection.name(), Some("termfilepickers"));
        assert_eq!(probe.scanned_dirs.len(), 1);
        assert_eq!(probe.loaded.len(), 1);
    }

    #[test]
    fn parses_1_22_interface_specific_wording() {
        let probe = parsed(&format!(
            "XDP: Using termfilepickers.portal for {FILE_CHOOSER_IFACE} (interface specific config)\n"
        ));

        match probe.selection {
            Selection::Backend { name, reason } => {
                assert_eq!(name, "termfilepickers");
                assert_eq!(reason, "interface specific config");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_1_16_wording_without_portal_suffix() {
        let probe = parsed(&format!("XDP: Using gtk for {FILE_CHOOSER_IFACE}\n"));
        assert_eq!(probe.selection.name(), Some("gtk"));
    }

    /// The glib warning prefix differs from the "XDP: " debug prefix.
    #[test]
    fn parses_last_resort_warning() {
        let probe = parsed(&format!(
            "(/usr/libexec/xdg-desktop-portal:2498730): xdg-desktop-portal-WARNING **: \
             08:30:56.973: Choosing gtk.portal for {FILE_CHOOSER_IFACE} as a last-resort fallback\n"
        ));

        match probe.selection {
            Selection::LastResort { name } => assert_eq!(name, "gtk"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_use_in_warning() {
        let probe = parsed(&format!(
            "(x:1): xdg-desktop-portal-WARNING **: 08:30:56.973: Choosing termfilechooser.portal \
             for {FILE_CHOOSER_IFACE} via the deprecated UseIn key\n"
        ));

        match probe.selection {
            Selection::UseIn { name } => assert_eq!(name, "termfilechooser"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn reports_none_when_interface_is_never_selected() {
        let probe = parsed("XDP: load portals from /usr/share/xdg-desktop-portal/portals\n");
        assert_eq!(probe.selection, Selection::None);
    }

    #[test]
    fn rejects_unrelated_output() {
        assert!(parse(Path::new("/x"), None, "some unrelated output\n").is_none());
    }
}
