use crate::config;

use anyhow::Result;

#[derive(Debug, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerOpenFileOptions {
    pub multiple: bool,
    pub directory: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFileOptions {
    pub current_folder: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFilesOptions {
    pub current_folder: Option<String>,
    pub files: Vec<String>,
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

    #[tracing::instrument(skip(options))]
    fn run_script(
        script_path: &std::path::Path,
        options: &impl serde::Serialize,
    ) -> Result<RunnerResult> {
        let out = std::process::Command::new(script_path)
            .arg(serde_json::to_string(options)?)
            .output()?;

        if !out.status.success() {
            anyhow::bail!(
                "Runner
             failed: {:?}",
                out
            );
        }

        let stdout = String::from_utf8(out.stdout)?;
        serde_json::from_str(&stdout).map_err(Into::into)
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
