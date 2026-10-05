use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub open_file_script_path: PathBuf,
    pub save_file_script_path: PathBuf,
    pub save_files_script_path: PathBuf,

    pub terminal_command: Vec<String>,
}

impl Config {
    /// Check if a script is valid by checking if it is a file that is executable.
    pub fn is_valid_script(script_path: &Path) -> Result<()> {
        if !script_path.is_file() {
            bail!("Not a file: {:?}", script_path);
        }

        if let Ok(metadata) = script_path.metadata() {
            if metadata.permissions().mode() & 0o111 == 0 {
                bail!("Not executable: {:?}", script_path);
            }
        }
        Ok(())
    }

    /// The scripts referenced by this config, paired with the option that declares them.
    pub fn scripts(&self) -> [(&'static str, &Path); 3] {
        [
            ("open_file_script_path", &self.open_file_script_path),
            ("save_file_script_path", &self.save_file_script_path),
            ("save_files_script_path", &self.save_files_script_path),
        ]
    }

    pub fn validate(self) -> Result<Self> {
        Self::is_valid_script(&self.open_file_script_path)?;
        Self::is_valid_script(&self.save_file_script_path)?;
        Self::is_valid_script(&self.save_files_script_path)?;

        Ok(self)
    }
}
