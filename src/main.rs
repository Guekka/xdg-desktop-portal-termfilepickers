use anyhow::Result;
use zbus::{connection, interface};

struct Picker;

#[interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl Picker {
    async fn open_file(&self, _multiple: bool, _directory: bool) {
        println!("open_file");
    }

    async fn open_files(&self) {
        println!("open_files");
    }

    async fn save_file(&self) {
        println!("save_file");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let picker = Picker {};
    let _conn = connection::Builder::session()?
        .name("org.freedesktop.impl.portal.desktop.termfilepickers")?
        .serve_at("/org/freedesktop/portal/desktop", picker)?
        .build()
        .await?;

    loop {
        // do something else, wait forever or timeout here:
        // handling D-Bus messages is done in the background
        std::future::pending::<()>().await;
    }
}
