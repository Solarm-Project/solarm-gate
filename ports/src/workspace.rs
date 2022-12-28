use curl::easy::{Handler, WriteError};
use miette::Diagnostic;
use sha2::Digest;
use std::{
    fs::DirBuilder,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum WorkspaceError {
    #[error(transparent)]
    #[diagnostic(code(wk::io))]
    IOError(#[from] std::io::Error),

    #[error("the url {0} is an invalid format it must have a filename at the end")]
    #[diagnostic(code(wk::url::invalid))]
    InvalidURLError(url::Url),
}

type Result<T> = miette::Result<T, WorkspaceError>;

#[derive(Debug)]
pub struct Workspace {
    path: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn get_download_dir(&self) -> PathBuf {
        self.path.join("downloads")
    }

    pub fn get_file_path(&self, url: url::Url) -> Result<PathBuf> {
        let download_dir = self.get_download_dir();
        if !download_dir.exists() {
            DirBuilder::new().recursive(true).create(&download_dir)?;
        }
        Ok(download_dir.join(
            Path::new(url.path())
                .file_name()
                .ok_or(WorkspaceError::InvalidURLError(url.clone()))?,
        ))
    }

    pub fn open_local_file(&self, url: url::Url) -> Result<DownloadFile> {
        let download_dir = self.get_download_dir();
        if !download_dir.exists() {
            DirBuilder::new().recursive(true).create(&download_dir)?;
        }
        DownloadFile::new(
            download_dir.join(
                Path::new(url.path())
                    .file_name()
                    .ok_or(WorkspaceError::InvalidURLError(url.clone()))?,
            ),
        )
    }

    pub fn get_name(&self) -> String {
        let name_path = self.path.file_name().unwrap();
        name_path.to_string_lossy().to_string()
    }
}

pub struct DownloadFile(PathBuf, std::fs::File, sha2::Sha512, Option<String>);

impl DownloadFile {
    fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(DownloadFile(
            path.as_ref().clone().to_path_buf(),
            std::fs::File::options()
                .read(true)
                .write(true)
                .create_new(true)
                .open(path)?,
            sha2::Sha512::new(),
            None,
        ))
    }

    pub fn get_hash(&mut self) -> String {
        format!("{:x}", self.2.clone().finalize())
    }

    pub fn get_path(&self) -> PathBuf {
        self.0.clone().to_path_buf()
    }

    #[allow(dead_code)]
    pub fn exists(&self) -> bool {
        self.0.exists()
    }
}

impl Handler for DownloadFile {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        let len = match self.1.write(data) {
            Ok(l) => l,
            Err(e) => {
                self.3 = Some(format!(
                    "error while downloading {} inside handler: {}",
                    self.0.display(),
                    e
                ));
                return Err(WriteError::Pause);
            }
        };
        self.2.update(data);
        Ok(len)
    }
}
