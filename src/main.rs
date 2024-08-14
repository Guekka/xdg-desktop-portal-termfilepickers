use std::{collections::HashMap, error::Error};

use anyhow::Result;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;
use zbus::{connection, interface, zvariant::OwnedValue};

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

#[derive(Debug, Default)]
struct Picker;

#[interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl Picker {
    #[tracing::instrument]
    #[dbus_interface(out_args("response", "results"))]
    async fn open_file(&self) -> (u32, HashMap<String, OwnedValue>) {
        println!("open_file");
        (666, HashMap::new())
    }

    #[tracing::instrument]
    #[dbus_interface(out_args("response", "results"))]
    async fn open_files(&self) -> (u32, HashMap<String, OwnedValue>) {
        println!("open_files");
        (0, HashMap::new())
    }

    #[tracing::instrument]
    #[dbus_interface(out_args("response", "results"))]
    async fn save_file(&self) -> (u32, HashMap<String, OwnedValue>) {
        println!("save_file");
        (0, HashMap::new())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_tracing()?;

    let picker = Picker::default();

    let _conn = connection::Builder::session()?
        .name("org.freedesktop.impl.portal.desktop.termfilepickers")?
        .serve_at("/org/freedesktop/portal/desktop", picker)?
        .build()
        .await?;

    log::info!("Service started");

    std::future::pending::<()>().await;

    Ok(())
}
