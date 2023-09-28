use crate::workspace::{DownloadFile, HasherKind, Workspace};
use bundle::{ArchiveSource, Bundle, SourceNode};
use clap::Subcommand;
use curl::easy::Easy2;
use miette::{IntoDiagnostic, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Subcommand)]
pub enum Sources {
    Archive {
        url: url::Url,
        hash: Option<String>,
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
            Self::Archive { url, .. } => write!(f, "archive {}", url),
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

enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

pub fn handle_add_source(wks: &Workspace, src: &Sources, pkg: &mut Bundle) -> Result<()> {
    let src_node = match src {
        Sources::Archive { url, hash } => {
            let mut archive_node = ArchiveSource {
                src: url.to_string(),
                ..Default::default()
            };

            if let Some(hash) = hash {
                let (kind, hash) = if let Some((kind, hash)) = hash.split_once(":") {
                    (kind, hash)
                } else {
                    ("sha256", hash.as_str())
                };
                if kind == "sha256" {
                    archive_node.sha256 = Some(hash.to_owned());
                } else if kind == "sha512" {
                    archive_node.sha512 = Some(hash.to_owned());
                }
            }

            if archive_node.sha256.is_none() && archive_node.sha256.is_none() {
                println!("No Archive hash specified on commandline need to download the file to calculate the hash");
                let mut easy =
                    Easy2::new(wks.open_or_truncate_local_file(url.clone(), HasherKind::Sha512)?);
                easy.get(true).into_diagnostic()?;
                easy.url(&url.to_string()).into_diagnostic()?;
                easy.progress(true).into_diagnostic()?;
                easy.perform().into_diagnostic()?;

                archive_node.sha512 = Some({ easy.get_mut() as &mut DownloadFile }.get_hash());
            }

            OneOrMany::One(SourceNode::Archive(archive_node))
        }
        Sources::Git { url, branch, tag } => OneOrMany::One(SourceNode::Git(bundle::GitSource {
            repository: url.to_string(),
            branch: branch.as_deref().map(|s| String::from(s)),
            tag: tag.as_deref().map(|s| String::from(s)),
            archive: None,
            must_stay_as_repo: None,
            directory: None,
        })),
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
            OneOrMany::One(SourceNode::File(bundle::FileSource::new(
                Path::new(&file_name).to_path_buf(),
                target_path.as_deref().map(|s| Path::new(s).to_path_buf()),
            )?))
        }
        Sources::Patch {
            local_path,
            drop_directories,
        } => {
            if local_path.is_file() {
                let file_name = local_path
                    .file_name()
                    .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                    .to_string_lossy()
                    .to_string();

                OneOrMany::One(SourceNode::Patch(bundle::PatchSource::new(
                    Path::new(&file_name).to_path_buf(),
                    drop_directories.clone(),
                )?))
            } else {
                let mut patch_vec = vec![];
                let read_dir_res = fs::read_dir(local_path).into_diagnostic()?;
                for entry in read_dir_res {
                    let entry = entry.into_diagnostic()?;
                    if entry.path().is_file() {
                        let file_name = entry
                            .path()
                            .file_name()
                            .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                            .to_owned();
                        patch_vec.push(SourceNode::Patch(bundle::PatchSource::new(
                            file_name,
                            drop_directories.clone(),
                        )?));
                    }
                }
                OneOrMany::Many(patch_vec)
            }
        }
        Sources::Overlay { local_path } => {
            let file_name = local_path
                .file_name()
                .ok_or(miette::miette!("no filename for {}", local_path.display()))?
                .to_string_lossy()
                .to_string();

            OneOrMany::One(SourceNode::Overlay(bundle::OverlaySource::new(Path::new(
                &file_name,
            ))?))
        }
    };

    match src_node {
        OneOrMany::Many(v) => {
            for n in v {
                pkg.add_source(n)?;
            }
            Ok(())
        }
        OneOrMany::One(o) => pkg.add_source(o),
    }
}
