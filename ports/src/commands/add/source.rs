use crate::workspace::{DownloadFile, Workspace};
use bundle::{ArchiveSource, Bundle, SourceNode};
use clap::Subcommand;
use curl::easy::Easy2;
use miette::{IntoDiagnostic, Result, WrapErr};
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum Sources {
    Archive {
        url: url::Url,
    },
    Git {
        url: url::Url,
        #[arg(short, long)]
        branch: Option<String>,
        #[arg(short, long)]
        tag: Option<String>,
    },
    File {
        local_path: PathBuf,
        #[arg(short, long = "target")]
        target_path: Option<String>,
    },
    Patch {
        local_path: PathBuf,
        #[arg(short, long)]
        drop_directories: Option<i64>,
    },
    Overlay {
        local_path: PathBuf,
    },
}

impl std::fmt::Display for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Archive { url } => write!(f, "archive {}", url),
            Self::Git { url, branch, tag } => write!(
                f,
                "git {} branch={} tag={}",
                url,
                branch.as_ref().unwrap_or(&String::from("main")),
                tag.as_ref().unwrap_or(&String::from("HEAD"))
            ),
            Self::File {
                local_path,
                target_path,
            } => write!(
                f,
                "file {} target={}",
                local_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(String::from("null")),
                target_path.as_ref().unwrap_or(&String::from("null"))
            ),
            Self::Patch { local_path, .. } => write!(f, "patch {}", local_path.display()),
            Self::Overlay { local_path } => write!(f, "patch {}", local_path.display()),
        }
    }
}

pub fn handle_add_source(wks: &Workspace, src: &Sources, pkg: &mut Bundle) -> Result<()> {
    let src_node = match src {
        Sources::Archive { url } => {
            let mut easy = Easy2::new(wks.open_local_file(url.clone())?);
            easy.get(true).into_diagnostic()?;
            easy.url(&url.to_string()).into_diagnostic()?;
            easy.progress(true).into_diagnostic()?;
            easy.perform().into_diagnostic()?;

            SourceNode::Archive(ArchiveSource {
                src: url.to_string(),
                sha512: { easy.get_mut() as &mut DownloadFile }.get_hash(),
            })
        }
        Sources::Git { url, branch, tag } => SourceNode::Git(bundle::GitSource {
            repository: url.to_string(),
            branch: branch.as_deref().map(|s| String::from(s)),
            tag: tag.as_deref().map(|s| String::from(s)),
            archive: None,
            must_stay_as_repo: None,
        }),
        Sources::File {
            local_path,
            target_path,
        } => {
            let bundle_path = pkg.get_path();
            let file_name = local_path
                .file_name()
                .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                .to_string_lossy()
                .to_string();
            let bundle_file_path = bundle_path.join(&file_name);
            std::fs::copy(local_path, bundle_file_path).into_diagnostic()?;
            SourceNode::File(bundle::FileSource::new(
                Path::new(&file_name).to_path_buf(),
                target_path.as_deref().map(|s| Path::new(s).to_path_buf()),
            )?)
        }
        Sources::Patch {
            local_path,
            drop_directories,
        } => {
            let bundle_path = pkg.get_path();
            let file_name = local_path
                .file_name()
                .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                .to_string_lossy()
                .to_string();
            let bundle_file_path = bundle_path.join(&file_name);
            std::fs::copy(local_path, bundle_file_path).into_diagnostic()?;
            SourceNode::Patch(bundle::PatchSource::new(
                Path::new(&file_name).to_path_buf(),
                drop_directories.clone(),
            )?)
        }
        Sources::Overlay { local_path } => {
            let bundle_path = pkg.get_path();
            let file_name = local_path
                .file_name()
                .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                .to_string_lossy()
                .to_string();
            let copy_opts = fs_extra::dir::CopyOptions::new();
            fs_extra::dir::copy(local_path, &bundle_path, &copy_opts)
                .into_diagnostic()
                .wrap_err("Error copying directory structure")?;
            SourceNode::Overlay(bundle::OverlaySource::new(Path::new(&file_name))?)
        }
    };

    pkg.add_source(src_node)
}
