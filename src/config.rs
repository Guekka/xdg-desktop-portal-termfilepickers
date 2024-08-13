use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub default_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_dir: Some(PathBuf::from(".")),
        }
    }
}
