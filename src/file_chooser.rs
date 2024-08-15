use std::collections::HashMap;

use anyhow::Result;
use zbus::zvariant::{self};

use crate::runner::{
    Runner, RunnerOpenFileOptions, RunnerResult, RunnerSaveFileOptions, RunnerSaveFilesOptions,
};

// types from https://github.com/pop-os/xdg-desktop-portal-cosmic/blob/41c1e7cfd0779db6005fc64798ac75b630332678/src/file_chooser.rs, thank you!

const PORTAL_RESPONSE_SUCCESS: u32 = 0;
const PORTAL_RESPONSE_CANCELLED: u32 = 1;
const PORTAL_RESPONSE_OTHER: u32 = 2;

#[derive(zvariant::Type)]
#[zvariant(signature = "(ua{sv})")]
enum PortalResponse<T: zvariant::Type + serde::Serialize> {
    Success(T),
    #[allow(dead_code)]
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

impl From<Result<FileChooserResult>> for PortalResponse<FileChooserResult> {
    fn from(res: Result<FileChooserResult>) -> Self {
        match res {
            Ok(res) => Self::Success(res),
            Err(err) => {
                tracing::error!("Error: {:?}", err);
                Self::Other
            }
        }
    }
}

pub type Choices = Vec<(String, String, Vec<(String, String)>, String)>;
pub type Filter = (String, Vec<(u32, String)>);
pub type Filters = Vec<Filter>;

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
#[allow(dead_code)]
pub struct OpenFileOptions {
    accept_label: Option<String>,
    modal: Option<bool>,
    multiple: Option<bool>,
    directory: Option<bool>,
    filters: Option<Filters>,
    current_filter: Option<Filter>,
    choices: Option<Choices>,
    current_folder: Option<Vec<u8>>,
}

impl From<OpenFileOptions> for RunnerOpenFileOptions {
    fn from(options: OpenFileOptions) -> Self {
        Self {
            multiple: options.multiple.unwrap_or(false),
            directory: options.directory.unwrap_or(false),
        }
    }
}

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
#[allow(dead_code)]
pub struct SaveFileOptions {
    accept_label: Option<String>,
    modal: Option<bool>,
    filters: Option<Filters>,
    current_filter: Option<Filter>,
    choices: Option<Choices>,
    current_name: Option<String>,
    current_folder: Option<Vec<u8>>,
    current_file: Option<Vec<u8>>,
}

impl From<SaveFileOptions> for RunnerSaveFileOptions {
    fn from(options: SaveFileOptions) -> Self {
        Self {
            current_folder: options
                .current_folder
                .map(|folder| String::from_utf8_lossy(folder.as_slice()).into_owned()),
        }
    }
}

#[derive(zvariant::DeserializeDict, zvariant::Type, Clone, Debug)]
#[zvariant(signature = "a{sv}")]
#[allow(dead_code)]
pub struct SaveFilesOptions {
    accept_label: Option<String>,
    modal: Option<bool>,
    choices: Option<Choices>,
    current_folder: Option<Vec<u8>>,
    files: Option<Vec<Vec<u8>>>,
}

impl From<SaveFilesOptions> for RunnerSaveFilesOptions {
    fn from(options: SaveFilesOptions) -> Self {
        Self {
            current_folder: options
                .current_folder
                .map(|folder| String::from_utf8_lossy(folder.as_slice()).into_owned()),
            files: options
                .files
                .map(|files| {
                    files
                        .into_iter()
                        .map(|file| String::from_utf8_lossy(file.as_slice()).into_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

#[derive(zvariant::SerializeDict, zvariant::Type, Debug, Default)]
#[zvariant(signature = "a{sv}")]
pub struct FileChooserResult {
    pub uris: Vec<String>,
    pub choices: Vec<(String, String)>,
    pub current_filter: Option<Filter>,
}

impl From<RunnerResult> for FileChooserResult {
    fn from(result: RunnerResult) -> Self {
        let uris = result.uris;

        Self {
            uris,
            ..Default::default()
        }
    }
}

impl From<Result<RunnerResult>> for PortalResponse<FileChooserResult> {
    fn from(res: Result<RunnerResult>) -> Self {
        match res {
            Ok(res) => {
                let res: FileChooserResult = res.into();
                if res.uris.is_empty() {
                    Self::Cancelled
                } else {
                    Self::Success(res)
                }
            }
            Err(err) => {
                tracing::error!("Error: {:?}", err);
                Self::Other
            }
        }
    }
}

#[derive(Debug)]
pub struct FileChooser {
    runner: Box<dyn Runner>,
}

impl FileChooser {
    pub fn new(runner: Box<dyn Runner>) -> Self {
        Self { runner }
    }
}

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
        self.runner
            .run_open_file(&RunnerOpenFileOptions::from(options))
            .into()
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
        self.runner
            .run_save_file(&RunnerSaveFileOptions::from(options))
            .into()
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
        self.runner
            .run_save_files(&RunnerSaveFilesOptions::from(options))
            .into()
    }
}
