use std::{collections::HashMap, ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

use zbus::zvariant::{self};

// types from https://github.com/pop-os/xdg-desktop-portal-cosmic/blob/41c1e7cfd0779db6005fc64798ac75b630332678/src/file_chooser.rs, thank you!

const PORTAL_RESPONSE_SUCCESS: u32 = 0;
const PORTAL_RESPONSE_CANCELLED: u32 = 1;
const PORTAL_RESPONSE_OTHER: u32 = 2;

#[derive(zvariant::Type)]
#[zvariant(signature = "(ua{sv})")]
enum PortalResponse<T: zvariant::Type + serde::Serialize> {
    Success(T),
    Cancelled,
    Other,
}

impl<T: zvariant::Type + serde::Serialize> serde::Serialize for PortalResponse<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Success(res) => (PORTAL_RESPONSE_SUCCESS, res).serialize(serializer),
            Self::Cancelled => (
                PORTAL_RESPONSE_CANCELLED,
                HashMap::<String, zvariant::Value>::new(),
            )
                .serialize(serializer),
            Self::Other => (
                PORTAL_RESPONSE_OTHER,
                HashMap::<String, zvariant::Value>::new(),
            )
                .serialize(serializer),
        }
    }
}
type Choices = Vec<(String, String, Vec<(String, String)>, String)>;
type Filter = (String, Vec<(u32, String)>);
type Filters = Vec<Filter>;

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
pub struct OpenFileOptions {
    accept_label: Option<String>,
    #[allow(dead_code)]
    modal: Option<bool>,
    multiple: Option<bool>,
    directory: Option<bool>,
    filters: Option<Filters>,
    current_filter: Option<Filter>,
    choices: Option<Choices>,
    current_folder: Option<Vec<u8>>,
}

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
pub struct SaveFileOptions {
    accept_label: Option<String>,
    #[allow(dead_code)]
    modal: Option<bool>,
    filters: Option<Filters>,
    current_filter: Option<Filter>,
    choices: Option<Choices>,
    current_name: Option<String>,
    current_folder: Option<Vec<u8>>,
    #[allow(dead_code)]
    current_file: Option<Vec<u8>>,
}

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
pub struct SaveFilesOptions {
    accept_label: Option<String>,
    #[allow(dead_code)]
    modal: Option<bool>,
    choices: Option<Choices>,
    current_folder: Option<Vec<u8>>,
    #[allow(dead_code)]
    files: Option<Vec<Vec<u8>>>,
}

#[derive(Clone, Debug)]
pub enum FileChooserOptions {
    OpenFile(OpenFileOptions),
    SaveFile(SaveFileOptions),
    SaveFiles(SaveFilesOptions),
}

impl FileChooserOptions {
    fn accept_label(&self) -> Option<String> {
        match self {
            Self::OpenFile(x) => x.accept_label.clone(),
            Self::SaveFile(x) => x.accept_label.clone(),
            Self::SaveFiles(x) => x.accept_label.clone(),
        }
    }

    fn choices(&self) -> Option<Choices> {
        match self {
            Self::OpenFile(x) => x.choices.clone(),
            Self::SaveFile(x) => x.choices.clone(),
            Self::SaveFiles(x) => x.choices.clone(),
        }
    }

    fn filters(&self) -> Option<Filters> {
        match self {
            Self::OpenFile(x) => x.filters.clone(),
            Self::SaveFile(x) => x.filters.clone(),
            Self::SaveFiles(_) => None,
        }
    }

    fn current_filter(&self) -> Option<Filter> {
        match self {
            Self::OpenFile(x) => x.current_filter.clone(),
            Self::SaveFile(x) => x.current_filter.clone(),
            Self::SaveFiles(_) => None,
        }
    }

    #[allow(dead_code)]
    fn modal(&self) -> bool {
        // Defaults to true
        match self {
            Self::OpenFile(x) => x.modal,
            Self::SaveFile(x) => x.modal,
            Self::SaveFiles(x) => x.modal,
        }
        .unwrap_or(true)
    }

    fn current_folder(&self) -> Option<PathBuf> {
        match self {
            Self::OpenFile(x) => x.current_folder.clone(),
            Self::SaveFile(x) => x.current_folder.clone(),
            Self::SaveFiles(x) => x.current_folder.clone(),
        }
        .map(|mut x| {
            // Trim leading NULs
            while x.last() == Some(&0) {
                x.pop();
            }
            PathBuf::from(OsString::from_vec(x))
        })
    }
}

#[derive(zvariant::SerializeDict, zvariant::Type)]
#[zvariant(signature = "a{sv}")]
pub struct FileChooserResult {
    uris: Vec<String>,
    choices: Vec<(String, String)>,
    current_filter: Option<Filter>,
}

#[derive(Debug, Default)]
pub struct FileChooser {}

#[zbus::interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl FileChooser {
    #[tracing::instrument]
    async fn open_file(
        &self,
        handle: zvariant::ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: OpenFileOptions,
    ) -> PortalResponse<FileChooserResult> {
        todo!();
    }

    #[tracing::instrument]
    async fn save_file(
        &self,
        handle: zvariant::ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: SaveFileOptions,
    ) -> PortalResponse<FileChooserResult> {
        todo!();
    }

    #[tracing::instrument]
    async fn save_files(
        &self,
        handle: zvariant::ObjectPath<'_>,
        app_id: &str,
        parent_window: &str,
        title: &str,
        options: SaveFilesOptions,
    ) -> PortalResponse<FileChooserResult> {
        todo!();
    }
}
