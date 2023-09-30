use crate::workspace::{Workspace, WorkspaceError};
use directories::ProjectDirs;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::{
    fs::DirBuilder,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("no project directory could be derived. Is this cli running on a supported OS?")]
    NoProjectDir,

    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

type Result<T> = miette::Result<T, Error>;

const QUALIFIER: &str = "org";
const ORG: &str = "solarm";
const APP_NAME: &str = "pkgdev";
const DEFAULT_WORKSPACE_DIR: &str = "wks";
const DEFAULT_OUTPUT_DIR_DIR: &str = "output";
const DEFAULT_REPO_DIR_DIR: &str = "repo";

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    current: Option<String>,
    base_path: Option<String>,
    output_dir: Option<String>,
    pub github_token: Option<GitHubToken>,
    search_path: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Option<Vec<String>>,
    pub expires_in: Option<u64>,
}

impl Settings {
    pub fn open() -> Result<Self> {
        let config_dir = Settings::get_or_create_config_dir()?;
        let config = config::Config::builder()
            .set_default("current", Some(DEFAULT_WORKSPACE_DIR))?
            .set_default("output", Some(DEFAULT_OUTPUT_DIR_DIR))?
            .set_default(
                "search_path",
                Some(vec![
                    "/opt/solarm/bin",
                    "/usr/gnu/bin",
                    "/usr/bin",
                    "/usr/sbin",
                    "/sbin",
                ]),
            )?
            .add_source(config::File::from(config_dir.join("config")).required(false))
            .build()?;

        Ok(config.try_deserialize()?)
    }

    fn get_or_create_config_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let config_dir = proj_dir.config_dir();
        if !config_dir.exists() {
            DirBuilder::new().recursive(true).create(config_dir)?;
        }
        Ok(config_dir.to_path_buf())
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Settings::get_or_create_config_dir()?;
        let mut file = std::fs::File::create(config_dir.join("config.json"))?;
        serde_json::to_writer(&mut file, &self)?;
        Ok(())
    }

    fn get_or_create_data_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let data_dir = proj_dir.data_dir();
        if !data_dir.exists() {
            DirBuilder::new().recursive(true).create(data_dir)?;
        }
        Ok(data_dir.to_path_buf())
    }

    pub fn get_or_create_archives_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let archive_dir = proj_dir.cache_dir().join("archives");
        if !archive_dir.exists() {
            DirBuilder::new().recursive(true).create(&archive_dir)?;
        }
        Ok(archive_dir.to_path_buf())
    }

    pub fn get_or_create_output_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let output_dir = proj_dir.data_dir().join(DEFAULT_OUTPUT_DIR_DIR);
        if !output_dir.exists() {
            DirBuilder::new().recursive(true).create(&output_dir)?;
        }
        Ok(output_dir.to_path_buf())
    }

    pub fn get_or_create_repo_dir() -> Result<PathBuf> {
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let repo_dir = proj_dir.data_dir().join(DEFAULT_REPO_DIR_DIR);
        if !repo_dir.exists() {
            DirBuilder::new().recursive(true).create(&repo_dir)?;
        }
        Ok(repo_dir.to_path_buf())
    }

    pub fn get_output_dir(&self) -> String {
        match &self.output_dir {
            Some(x) => x.to_string(),
            None => DEFAULT_OUTPUT_DIR_DIR.to_owned(),
        }
    }

    pub fn get_search_path(&self) -> Vec<String> {
        match &self.search_path {
            Some(x) => x.to_vec(),
            None => vec![
                "/opt/solarm/bin".into(),
                "/usr/gnu/bin".into(),
                "/usr/bin".into(),
                "/usr/sbin".into(),
                "/sbin".into(),
            ],
        }
    }

    pub fn add_path_to_search(&mut self, value: String) {
        if let Some(path) = &mut self.search_path {
            path.push(value);
        } else {
            self.search_path = Some(vec![value]);
        };
    }

    pub fn remove_path_from_search(&mut self, value: String) {
        if let Some(path) = &mut self.search_path {
            self.search_path = Some(path.clone().into_iter().filter(|e| e != &value).collect());
        } else {
            self.search_path = Some(vec![value]);
        };
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
        let workspaces = std::fs::read_dir(&data_dir)?
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
        let proj_dir = ProjectDirs::from(QUALIFIER, ORG, APP_NAME).ok_or(Error::NoProjectDir)?;
        let cache_dir = proj_dir.cache_dir();
        if !cache_dir.exists() {
            DirBuilder::new().recursive(true).create(cache_dir)?;
        }
        Ok(cache_dir.to_path_buf())
    }

    pub fn change_current_workspace(&mut self, name: &str) -> Result<Workspace> {
        let data_dir = Self::get_or_create_data_dir()?;
        let wks_path = data_dir.join(&name);
        if !wks_path.exists() {
            DirBuilder::new().recursive(true).create(&wks_path)?;
        }

        self.current = Some(name.clone().to_string());
        self.save()?;

        Ok(Workspace::new(wks_path)?)
    }
}
