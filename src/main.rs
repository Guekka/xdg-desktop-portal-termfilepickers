use std::error::Error;

use anyhow::Result;
use file_chooser::FileChooser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;
use zbus::connection;

mod file_chooser;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_tracing()?;

    let picker = FileChooser::default();

    let _conn = connection::Builder::session()?
        .name("org.freedesktop.impl.portal.desktop.termfilepickers")?
        .serve_at("/org/freedesktop/portal/desktop", picker)?
        .build()
        .await?;

    log::info!("Service started");

    std::future::pending::<()>().await;

    Ok(())
}
