use crate::config::{self, Config};

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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerOpenFileOptions {
    pub multiple: bool,
    pub directory: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFileOptions {
    pub recommended_path: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RunnerSaveFilesOptions {
    pub current_folder: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum SpecificRunnerArguments {
    OpenFile(RunnerOpenFileOptions),
    SaveFile(RunnerSaveFileOptions),
    SaveFiles(RunnerSaveFilesOptions),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct RunnerArguments {
    #[serde(flatten)]
    specific: SpecificRunnerArguments,
    out_file: String,
    termcmd: String,
}

impl RunnerArguments {
    fn new(config: &Config, specific: SpecificRunnerArguments) -> Self {
        Self {
            specific,
            out_file: temp_path(),
            termcmd: config.terminal_command.clone(),
        }
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
    pub(crate) fn new(config: config::Config) -> Self {
        Self { config }
    }

    fn parse_result(out_file: &str) -> Result<RunnerResult> {
        let content = std::fs::read_to_string(out_file)?;

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
        options: &RunnerArguments,
    ) -> Result<RunnerResult> {
        let out = std::process::Command::new(script_path)
            .arg(serde_json::to_string(options)?)
            .output()?;

        if !out.status.success() {
            anyhow::bail!("Runner failed: {:?}", out);
        }

        Self::parse_result(&options.out_file).with_context(|| {
            format!(
                "Script did not produce a valid output file: {}.\n
                     Script standard output was: {:?}",
                options.out_file, out
            )
        })
    }
}

impl Runner for ConfigRunner {
    fn run_open_file(&self, options: &RunnerOpenFileOptions) -> Result<RunnerResult> {
        let args = RunnerArguments::new(
            &self.config,
            SpecificRunnerArguments::OpenFile(options.clone()),
        );
        Self::run_script(&self.config.open_file_script_path, &args)
    }

    fn run_save_file(&self, options: &RunnerSaveFileOptions) -> Result<RunnerResult> {
        let args = RunnerArguments::new(
            &self.config,
            SpecificRunnerArguments::SaveFile(options.clone()),
        );
        Self::run_script(&self.config.save_file_script_path, &args)
    }

    fn run_save_files(&self, options: &RunnerSaveFilesOptions) -> Result<RunnerResult> {
        let args = RunnerArguments::new(
            &self.config,
            SpecificRunnerArguments::SaveFiles(options.clone()),
        );
        Self::run_script(&self.config.save_files_script_path, &args)
    }
}
