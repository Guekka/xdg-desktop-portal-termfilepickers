use crate::config;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

const FILE_PROTOCOL: &str = "file://";

fn temp_path() -> String {
    NamedTempFile::new()
        .expect("Failed to create temp file")
        .into_temp_path()
        .to_string_lossy()
        .to_string()
}

trait RunnerOptions : serde::Serialize {
    fn out_file(&self) -> &str;
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerOpenFileOptions {
    pub multiple: bool,
    pub directory: bool,
    out_file: String,
}

impl RunnerOpenFileOptions {
    pub fn new(multiple: bool, directory: bool) -> Self {
        Self {
            multiple,
            directory,
            out_file: temp_path(),
        }
    }
}

impl RunnerOptions for RunnerOpenFileOptions {
    fn out_file(&self) -> &str {
        &self.out_file
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFileOptions {
    pub current_folder: Option<String>,
    pub out_file: String,
}

impl RunnerSaveFileOptions {
    pub fn new(current_folder: Option<String>) -> Self {
        Self {
            current_folder,
            out_file: temp_path(),
        }
    }
}

impl RunnerOptions for RunnerSaveFileOptions {
    fn out_file(&self) -> &str {
        &self.out_file
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFilesOptions {
    pub current_folder: Option<String>,
    pub files: Vec<String>,
    pub out_file: String,
}

impl RunnerSaveFilesOptions {
    pub fn new(current_folder: Option<String>, files: Vec<String>) -> Self {
        Self {
            current_folder,
            files,
            out_file: temp_path(),
        }
    }
}

impl RunnerOptions for RunnerSaveFilesOptions {
    fn out_file(&self) -> &str {
        &self.out_file
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerResult {
    // for now, only support uris. TODO: filters?
    pub uris: Vec<String>,
}

pub trait Runner: std::fmt::Debug + Send + Sync {
    fn run_open_file(&self, options: &RunnerOpenFileOptions) -> Result<RunnerResult>;
    fn run_save_file(&self, options: &RunnerSaveFileOptions) -> Result<RunnerResult>;
    fn run_save_files(&self, options: &RunnerSaveFilesOptions) -> Result<RunnerResult>;
}

#[derive(Debug)]
pub struct ConfigRunner {
    config: config::Config,
}

impl ConfigRunner {
    pub(crate) fn new(config: config::Config) -> Result<Self> {
        Ok(Self { config })
    }

    fn parse_result(out_file: &str) -> Result<RunnerResult> {
        let content = std::fs::read_to_string(out_file)
            .with_context(|| format!("Failed to read output file: {}", out_file))?;

        // newline separated list of files
        let uris = content
            .lines()
            .map(str::to_string)
            .map(|uri| {
                if uri.starts_with(FILE_PROTOCOL) {
                    uri
                } else {
                    format!("{}{}", FILE_PROTOCOL, uri)
                }
            })
            .collect();

        Ok(RunnerResult { uris })
    }

    #[tracing::instrument(skip(options))]
    fn run_script(
        script_path: &std::path::Path,
        options: &impl RunnerOptions,
    ) -> Result<RunnerResult> {
        let out = std::process::Command::new(script_path)
            .arg(serde_json::to_string(options)?)
            .output()?;

        if !out.status.success() {
            anyhow::bail!("Runner failed: {:?}", out);
        }

        Self::parse_result(&options.out_file())
   }
}

impl Runner for ConfigRunner {
    fn run_open_file(&self, options: &RunnerOpenFileOptions) -> Result<RunnerResult> {
        Self::run_script(&self.config.open_file_script_path, options)
    }

    fn run_save_file(&self, options: &RunnerSaveFileOptions) -> Result<RunnerResult> {
        Self::run_script(&self.config.save_file_script_path, options)
    }

    fn run_save_files(&self, options: &RunnerSaveFilesOptions) -> Result<RunnerResult> {
        Self::run_script(&self.config.save_files_script_path, options)
    }
}
