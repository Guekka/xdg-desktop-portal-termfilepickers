use crate::state::XdpwState;
use anyhow::{Context, Result};
use std::fs::remove_file;
use std::path::Path;
use zbus::zvariant::Value;

#[derive(Debug)]
pub struct FileChooserOptions<'a> {
    pub writing: bool,
    pub multiple: bool,
    pub directory: bool,
    pub path: Option<&'a str>,
}

pub async fn exec_filechooser(
    state: &XdpwState,
    options: FileChooserOptions<'_>,
) -> Result<Vec<String>> {
    let args = vec![
        ("multiple", options.multiple.into()),
        ("directory", options.directory.into()),
        ("writing", options.writing.into()),
        ("path", options.path.unwrap_or("").into()),
    ];

    let response = state
        .connection
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.FileChooser"),
            "OpenFile",
            &args,
        )
        .await
        .context("Failed to call D-Bus method")
        .unwrap(); // FIXME: Handle error

    // Extract file URIs from the response
    let selected_files = match response {
        Value::Array(file_uris) => file_uris
            .iter()
            .filter_map(|uri| uri.as_str())
            .map(|uri| format!("file://{}", uri))
            .collect(),
        _ => vec![],
    };

    Ok(selected_files)
}

pub async fn method_open_file(state: &XdpwState, multiple: bool, directory: bool) -> Result<()> {
    let options = FileChooserOptions {
        writing: false,
        multiple,
        directory,
        path: None,
    };

    let selected_files = exec_filechooser(state, options).await?;

    println!("Number of selected files: {}", selected_files.len());
    for (i, file) in selected_files.iter().enumerate() {
        println!("{}. {}", i, file);
    }

    Ok(())
}

pub async fn method_save_file(
    state: &XdpwState,
    current_name: &str,
    current_folder: Option<&str>,
) -> Result<()> {
    let folder =
        current_folder.unwrap_or_else(|| state.config.default_dir.as_deref().unwrap_or("."));

    let mut path = format!("{}/{}", folder, current_name);
    while Path::new(&path).exists() {
        path.push('_');
    }

    let options = FileChooserOptions {
        writing: true,
        multiple: false,
        directory: false,
        path: Some(&path),
    };

    let selected_files = exec_filechooser(state, options).await?;
    if selected_files.is_empty() {
        remove_file(&path)?;
        return Err(anyhow::anyhow!("No file selected"));
    }

    println!("Number of selected files: {}", selected_files.len());
    for (i, file) in selected_files.iter().enumerate() {
        println!("{}. {}", i, file);
    }

    Ok(())
}
