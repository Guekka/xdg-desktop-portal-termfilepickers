use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub open_file_script_path: PathBuf,
    pub save_file_script_path: PathBuf,
    pub save_files_script_path: PathBuf,
}
