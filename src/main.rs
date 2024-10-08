mod config;
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
struct Args {
    #[arg(short, long)]
    config_path: Option<String>,
}

fn load_config(args: &Args) -> Result<config::Config> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("termfilepickers")?;
    let config_path = {
        let user_path = args.config_path.as_deref();
        if let Some(path) = user_path {
            PathBuf::from(path)
        } else {
            xdg_dirs
                .find_config_file("config.toml")
                .ok_or(anyhow::anyhow!(
                    "Config file not found. Use --config-path to specify the path"
                ))?
        }
    };

    tracing::info!("Loading config from {:?}", config_path);
    let content = std::fs::read_to_string(&config_path).context("Failed to read config file")?;

    toml::from_str(&content)
        .context("Failed to parse config file")
        .and_then(Config::validate)
        .context("Failed to validate config")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_tracing()?;

    let config = load_config(&Args::parse())?;
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
