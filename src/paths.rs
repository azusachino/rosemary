use directories::ProjectDirs;
use std::env;
use std::path::PathBuf;

pub struct RosemaryPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub kb_dir: PathBuf,
}

impl RosemaryPaths {
    pub fn resolve() -> Self {
        let home = env::var("ROSEMARY_HOME").map(PathBuf::from).ok();

        let proj_dirs = ProjectDirs::from("me", "azusachino", "rosemary");

        let data_dir = home.clone().unwrap_or_else(|| {
            proj_dirs
                .as_ref()
                .map(|d| d.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".rosemary/data"))
        });

        let config_dir = home.clone().unwrap_or_else(|| {
            proj_dirs
                .as_ref()
                .map(|d| d.config_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".rosemary/config"))
        });

        let kb_dir = home.unwrap_or_else(|| {
            proj_dirs
                .as_ref()
                .map(|d| d.data_dir().join("topics"))
                .unwrap_or_else(|| PathBuf::from("kb/topics"))
        });

        Self {
            data_dir,
            config_dir,
            kb_dir,
        }
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("rosemary.db")
    }
}
