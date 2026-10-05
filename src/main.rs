mod config;
mod doctor;
mod file_chooser;
mod runner;

use std::{error::Error, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use file_chooser::FileChooser;
use runner::ConfigRunner;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;
use zbus::connection;

use crate::config::Config;

pub(crate) fn setup_tracing() -> Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive("zbus=error".parse()?);

    Registry::default()
        .with(env_filter)
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();

    Ok(())
}

#[derive(Debug, clap::Parser)]
#[command(
    version,
    about = "A FileChooser XDG desktop portal backed by terminal file managers"
)]
struct Args {
    #[arg(short, long, global = true)]
    config_path: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Serve the portal. This is the default when no subcommand is given.
    Serve,
    /// Check that the portal is correctly set up.
    Doctor {
        /// Also run the configured open-file script, which opens a real picker.
        #[arg(long)]
        run_scripts: bool,
    },
}

/// Locate the config file, without reading it.
fn find_config_path(args: &Args) -> Result<PathBuf> {
    if let Some(path) = args.config_path.as_deref() {
        return Ok(PathBuf::from(path));
    }

    let xdg_dirs = xdg::BaseDirectories::with_prefix("termfilepickers")?;
    xdg_dirs
        .find_config_file("config.toml")
        .ok_or(anyhow::anyhow!(
            "Config file not found. Use --config-path to specify the path"
        ))
}

fn load_config_at(config_path: &std::path::Path) -> Result<config::Config> {
    tracing::info!("Loading config from {:?}", config_path);
    let content = std::fs::read_to_string(config_path).context("Failed to read config file")?;

    toml::from_str(&content)
        .context("Failed to parse config file")
        .and_then(Config::validate)
        .context("Failed to validate config")
}

fn load_config(args: &Args) -> Result<config::Config> {
    load_config_at(&find_config_path(args)?)
}

/// `doctor` reports problems instead of failing on them, so it tolerates both a
/// missing and an invalid config.
fn run_doctor(args: &Args, run_scripts: bool) -> i32 {
    let config_path = find_config_path(args);

    let (path, error) = match &config_path {
        Ok(path) => (Some(path.as_path()), None),
        Err(err) => (None, Some(err)),
    };

    let (report, status) = doctor::run(doctor::DoctorOptions {
        config_path: path,
        config_error: error,
        run_scripts,
    });

    print!("{report}");

    doctor::exit_code(status)
}

fn main() -> Result<(), Box<dyn Error>> {
    setup_tracing()?;

    let args = Args::parse();

    // doctor uses the blocking zbus API, so it must stay outside the async runtime
    if let Some(Command::Doctor { run_scripts }) = &args.command {
        std::process::exit(run_doctor(&args, *run_scripts));
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(&args))
}

async fn serve(args: &Args) -> Result<(), Box<dyn Error>> {
    let config = load_config(args)?;
    let runner = Box::new(ConfigRunner::new(config));
    let picker = FileChooser::new(runner);

    let _conn = connection::Builder::session()?
        .name("org.freedesktop.impl.portal.desktop.termfilepickers")
        .context("Failed to create dbus connection")?
        .serve_at("/org/freedesktop/portal/desktop", picker)
        .context("Failed to serve dbus service")?
        .build()
        .await
        .context("Failed to build dbus connection")?;

    log::info!("Service started");

    std::future::pending::<()>().await;

    Ok(())
}
