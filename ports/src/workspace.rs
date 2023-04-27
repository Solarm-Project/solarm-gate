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
        if !path.as_ref().exists() {
            DirBuilder::new().recursive(true).create(path.as_ref())?;
        }
        let full_path = std::fs::canonicalize(path.as_ref())?;
        Ok(Self { path: full_path })
    }

    pub fn get_or_create_download_dir(&self) -> Result<PathBuf> {
        let download_dir = self.path.join("downloads");
        if !download_dir.exists() {
            DirBuilder::new().recursive(true).create(&download_dir)?;
        }
        Ok(download_dir)
    }

    pub fn get_file_path(&self, url: url::Url) -> Result<PathBuf> {
        let download_dir = self.get_or_create_download_dir()?;
        Ok(download_dir.join(
            Path::new(url.path())
                .file_name()
                .ok_or(WorkspaceError::InvalidURLError(url.clone()))?,
        ))
    }

    pub fn open_local_file(&self, url: url::Url, hasher_kind: HasherKind) -> Result<DownloadFile> {
        let download_dir = self.get_or_create_download_dir()?;
        DownloadFile::new(
            download_dir.join(
                Path::new(url.path())
                    .file_name()
                    .ok_or(WorkspaceError::InvalidURLError(url.clone()))?,
            ),
            hasher_kind,
        )
    }

    pub fn get_name(&self) -> String {
        let name_path = self.path.file_name().unwrap();
        name_path.to_string_lossy().to_string()
    }

    pub fn get_or_create_build_dir(&self) -> Result<PathBuf> {
        let p = self.path.join("build");
        if !p.exists() {
            DirBuilder::new().recursive(true).create(&p)?;
        }
        Ok(p)
    }

    pub fn get_or_create_prototype_dir(&self) -> Result<PathBuf> {
        let p = self.path.join("proto");
        if !p.exists() {
            DirBuilder::new().recursive(true).create(&p)?;
        }
        Ok(p)
    }

    pub fn get_or_create_manifest_dir(&self) -> Result<PathBuf> {
        let p = self.path.join("manifests");
        if !p.exists() {
            DirBuilder::new().recursive(true).create(&p)?;
        }
        Ok(p)
    }
}

#[allow(dead_code)]
pub enum HasherKind {
    Sha256,
    Sha512,
}

pub struct DownloadFile {
    path: PathBuf,
    handle: std::fs::File,
    hasher512: sha2::Sha512,
    hasher256: sha2::Sha256,
    hasher_kind: HasherKind,
    error: Option<String>,
}

impl DownloadFile {
    fn new<P: AsRef<Path>>(path: P, kind: HasherKind) -> Result<Self> {
        Ok(DownloadFile {
            path: path.as_ref().clone().to_path_buf(),
            handle: std::fs::File::options()
                .read(true)
                .write(true)
                .create_new(true)
                .open(path)?,
            hasher_kind: kind,
            hasher512: sha2::Sha512::new(),
            hasher256: sha2::Sha256::new(),
            error: None,
        })
    }

    pub fn get_hash(&mut self) -> String {
        match self.hasher_kind {
            HasherKind::Sha256 => format!("{:x}", self.hasher256.clone().finalize()),
            HasherKind::Sha512 => format!("{:x}", self.hasher512.clone().finalize()),
        }
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone().to_path_buf()
    }

    #[allow(dead_code)]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

impl Handler for DownloadFile {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        let len = match self.handle.write(data) {
            Ok(l) => l,
            Err(e) => {
                self.error = Some(format!(
                    "error while downloading {} inside handler: {}",
                    self.path.display(),
                    e
                ));
                return Err(WriteError::Pause);
            }
        };
        match self.hasher_kind {
            HasherKind::Sha256 => self.hasher256.update(data),
            HasherKind::Sha512 => self.hasher512.update(data),
        }

        Ok(len)
    }
}
