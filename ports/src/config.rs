use directories::ProjectDirs;
use miette::{IntoDiagnostic, Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::{
    fs::{DirBuilder, File},
    path::{Path, PathBuf},
};

use crate::workspace::Workspace;

const QUALIFIER: &str = "org";
const ORG: &str = "solarm";
const APP_NAME: &str = "ports";
const NO_PROJECT_DIR_ERR_STR: &str =
    "no project directory could be derived. Is this cli running on a supported OS?";
const DEFAULT_WORKSPACE_DIR: &str = "wks";
const DEFAULT_OUTPUT_DIR_DIR: &str = "output";
const DEFAULT_REPO_DIR_DIR: &str = "repo";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    current: Option<String>,
    base_path: Option<String>,
    output_dir: Option<String>,
    pub github_token: Option<GitHubToken>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Option<Vec<String>>,
    pub expires_in: Option<u64>,
}

impl Config {
    pub fn open() -> Result<Self> {
        let config_file = Self::get_or_create_config_file_handle(false)?;
        let config: Self = match serde_json::from_reader(config_file) {
            Ok(v) => Ok(v),
            Err(x) => {
                if x.is_eof() {
                    Ok(Config {
                        current: Some(DEFAULT_WORKSPACE_DIR.clone().to_string()),
                        base_path: None,
                        output_dir: None,
                        github_token: None,
                    })
                } else {
                    Err(x)
                }
            }
        }
        .into_diagnostic()
        .wrap_err("could not read config file")?;

        Ok(config)
    }

    fn get_or_create_config_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let config_dir = proj_dir.config_dir();
        if !config_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(config_dir)
                .into_diagnostic()?;
        }
        Ok(config_dir.to_path_buf())
    }

    fn get_or_create_config_file_handle(write: bool) -> Result<File> {
        let config_dir = Self::get_or_create_config_dir()?;
        let config_file_path = config_dir.join("config.json");
        if !config_file_path.exists() {
            let _ = File::create(&config_file_path).into_diagnostic()?;
        }
        if write {
            Ok(File::options()
                .read(true)
                .write(true)
                .truncate(true)
                .open(&config_file_path)
                .into_diagnostic()?)
        } else {
            Ok(File::open(&config_file_path).into_diagnostic()?)
        }
    }

    pub fn save(&self) -> Result<()> {
        let mut config_file = Self::get_or_create_config_file_handle(true)?;
        serde_json::to_writer(&mut config_file, self).into_diagnostic()?;
        Ok(())
    }

    fn get_or_create_data_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let data_dir = proj_dir.data_dir();
        if !data_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(data_dir)
                .into_diagnostic()?;
        }
        Ok(data_dir.to_path_buf())
    }

    pub fn get_or_create_archives_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let archive_dir = proj_dir.cache_dir().join("archives");
        if !archive_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&archive_dir)
                .into_diagnostic()?;
        }
        Ok(archive_dir.to_path_buf())
    }

    pub fn get_or_create_output_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let output_dir = proj_dir.data_dir().join(DEFAULT_OUTPUT_DIR_DIR);
        if !output_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&output_dir)
                .into_diagnostic()?;
        }
        Ok(output_dir.to_path_buf())
    }

    pub fn get_or_create_repo_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let repo_dir = proj_dir.data_dir().join(DEFAULT_REPO_DIR_DIR);
        if !repo_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&repo_dir)
                .into_diagnostic()?;
        }
        Ok(repo_dir.to_path_buf())
    }

    pub fn get_workspace_from(&self, name: &str) -> Result<Workspace> {
        let base_path = if let Some(base_path) = &self.base_path {
            Path::new(base_path).to_path_buf()
        } else {
            Self::get_or_create_data_dir()?
        };

        let wks = Workspace::new(base_path.join(name))?;

        Ok(wks)
    }

    pub fn get_current_wks(&self) -> Result<Workspace> {
        let current = if let Some(current) = &self.current {
            current.clone()
        } else {
            String::from(DEFAULT_WORKSPACE_DIR)
        };

        let base_path = if let Some(base_path) = &self.base_path {
            Path::new(base_path).to_path_buf()
        } else {
            Self::get_or_create_data_dir()?
        };

        let wks = Workspace::new(base_path.join(&current))?;

        Ok(wks)
    }

    pub fn list_workspaces() -> Result<Vec<String>> {
        let data_dir = Self::get_or_create_data_dir()?;
        let workspaces = std::fs::read_dir(&data_dir)
            .into_diagnostic()?
            .into_iter()
            .map(|e| {
                e.unwrap()
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<String>>();
        if workspaces.len() == 0 {
            Ok(vec![String::from(DEFAULT_WORKSPACE_DIR)])
        } else {
            Ok(workspaces)
        }
    }

    #[allow(dead_code)]
    pub fn get_or_create_cache_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME)
            .ok_or(miette::miette!(NO_PROJECT_DIR_ERR_STR))?;
        let cache_dir = proj_dir.cache_dir();
        if !cache_dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(cache_dir)
                .into_diagnostic()?;
        }
        Ok(cache_dir.to_path_buf())
    }

    pub fn change_current_workspace(&mut self, name: &str) -> Result<Workspace> {
        let data_dir = Self::get_or_create_data_dir()?;
        let wks_path = data_dir.join(&name);
        if !wks_path.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&wks_path)
                .into_diagnostic()?;
        }

        self.current = Some(name.clone().to_string());
        self.save()?;

        Ok(Workspace::new(wks_path)?)
    }
}
