//! Rendering of the `doctor` results.

use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Everything is as it should be.
    Ok,
    /// Not fatal, but likely to surprise the user.
    Warn,
    /// The portal cannot work in this state.
    Fail,
}

impl Status {
    fn marker(self) -> &'static str {
        match self {
            Status::Ok => "[ ok ]",
            Status::Warn => "[warn]",
            Status::Fail => "[fail]",
        }
    }
}

#[derive(Debug)]
pub struct Check {
    pub name: String,
    pub status: Status,
    /// Short result description, shown next to the status marker.
    pub detail: String,
    /// Extra context, always shown (one line each).
    pub notes: Vec<String>,
    /// How to fix the problem. Only shown for `Warn` and `Fail`.
    pub hints: Vec<String>,
}

impl Check {
    pub fn new(name: impl Into<String>, status: Status, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status,
            detail: detail.into(),
            notes: Vec::new(),
            hints: Vec::new(),
        }
    }

    pub fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Ok, detail)
    }

    pub fn warn(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Warn, detail)
    }

    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(name, Status::Fail, detail)
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_notes<I, S>(mut self, notes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.notes.extend(notes.into_iter().map(Into::into));
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Section {
    pub title: String,
    pub checks: Vec<Check>,
}

impl Report {
    pub fn push(&mut self, title: impl Into<String>, checks: Vec<Check>) {
        self.sections.push(Section {
            title: title.into(),
            checks,
        });
    }

    pub fn worst_status(&self) -> Status {
        self.checks()
            .map(|check| check.status)
            .max()
            .unwrap_or(Status::Ok)
    }

    fn checks(&self) -> impl Iterator<Item = &Check> {
        self.sections.iter().flat_map(|section| &section.checks)
    }

    fn count(&self, status: Status) -> usize {
        self.checks().filter(|check| check.status == status).count()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        for section in &self.sections {
            let _ = writeln!(out, "{}", section.title);

            for check in &section.checks {
                let _ = writeln!(
                    out,
                    "  {} {}: {}",
                    check.status.marker(),
                    check.name,
                    check.detail
                );

                for note in &check.notes {
                    let _ = writeln!(out, "         {note}");
                }

                if check.status != Status::Ok {
                    for hint in &check.hints {
                        let _ = writeln!(out, "         -> {hint}");
                    }
                }
            }

            out.push('\n');
        }

        let (ok, warn, fail) = (
            self.count(Status::Ok),
            self.count(Status::Warn),
            self.count(Status::Fail),
        );

        let _ = writeln!(out, "{ok} passed, {warn} warning(s), {fail} error(s)");

        let verdict = match self.worst_status() {
            Status::Ok => "Everything looks good, as far as these checks can tell.",
            Status::Warn => "Mostly fine, but some things deserve a look.",
            Status::Fail => "Something looks broken: the file picker probably will not work.",
        };
        let _ = writeln!(out, "{verdict}");

        out.push('\n');
        out.push_str(DISCLAIMER);

        out
    }
}

/// Shown on every run. These checks reimplement and screen-scrape behaviour owned
/// by xdg-desktop-portal, so a confident-looking verdict can still be wrong, and
/// acting on a wrong hint can make things worse.
const DISCLAIMER: &str = "\
Note: these checks partly reimplement and partly parse the debug output of
xdg-desktop-portal, so they can be wrong or out of date with your version, in
either direction. They were largely written by an LLM and tested on few setups.
Use this as a starting point, not as proof, and review any suggested command
before running it. Authoritative: `xdg-desktop-portal -vr` and portals.conf(5).
";
