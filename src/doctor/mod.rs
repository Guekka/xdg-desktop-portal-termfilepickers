//! `doctor`: checks that the portal is set up correctly.
//!
//! The failure mode this exists for is that nothing is obviously broken: the
//! service runs, the config is valid, and applications still open their own file
//! picker. The cause is usually somewhere between xdg-desktop-portal's backend
//! selection and the toolkit settings of the calling application, so doctor
//! looks at the whole chain rather than at our own files only.

mod checks;
mod install;
mod keyfile;
mod portals;
mod probe;
mod report;
mod xdg;

use std::path::Path;

use report::{Report, Status};

pub struct DoctorOptions<'a> {
    /// The config file that was located, if any.
    pub config_path: Option<&'a Path>,
    /// Error from loading the config at startup, used for extra context.
    pub config_error: Option<&'a anyhow::Error>,
    /// Run the configured open-file script, which opens a real picker.
    pub run_scripts: bool,
}

/// Runs every check and returns the rendered report along with the worst status.
pub fn run(options: DoctorOptions<'_>) -> (String, Status) {
    let mut report = Report::default();

    let environment = portals::PortalEnvironment::discover();

    report.push("Session", checks::check_session(&environment));

    let config = checks::check_config(options.config_path, options.config_error);
    report.push("Configuration", config.checks);

    report.push("Portal backend", checks::check_portal_file(&environment));

    let probed = probe::probe();
    report.push(
        "Portal configuration",
        checks::check_portal_config(&environment, probed.is_ok()),
    );
    report.push(
        "Backend selection",
        checks::check_probe(&probed, &environment),
    );
    report.push("Running services", checks::check_dbus());
    report.push(
        "Application integration",
        checks::check_toolkit_integration(),
    );

    if options.run_scripts {
        match &config.config {
            Some(config) => report.push("Script execution", checks::check_script_run(config)),
            None => report.push(
                "Script execution",
                vec![report::Check::warn(
                    "open_file script run",
                    "skipped, the config could not be loaded",
                )],
            ),
        }
    }

    (report.render(), report.worst_status())
}

/// Exit code for a rendered report: non-zero when a check failed.
pub fn exit_code(status: Status) -> i32 {
    match status {
        Status::Ok | Status::Warn => 0,
        Status::Fail => 1,
    }
}

#[cfg(test)]
mod tests;
