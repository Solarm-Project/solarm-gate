use std::{
    fs::{read_dir, DirBuilder},
    path::{Path, PathBuf},
    process::Command,
};

use bundle::SourceSection;
use miette::{IntoDiagnostic, Result};

use crate::{config::Config, derive_source_name, workspace::Workspace};

pub fn unpack_sources<P: AsRef<Path>>(
    wks: &Workspace,
    package_name: String,
    bundle_path: P,
    sources: &[SourceSection],
) -> Result<()> {
    let bundle_path = bundle_path.as_ref();
    let build_dir = wks.get_or_create_build_dir()?;
    std::env::set_current_dir(&build_dir).into_diagnostic()?;

    for source in sources {
        let unpack_name = derive_source_name(package_name.clone(), &source);
        let unpack_path = build_dir.join(&unpack_name);

        for src in &source.sources {
            match src {
                bundle::SourceNode::Archive(archive) => {
                    let src_url: url::Url = archive.src.parse().into_diagnostic()?;

                    let local_file = wks.get_file_path(src_url.clone())?;

                    let file_name = local_file.file_name().ok_or(miette::miette!("Archive must have a file_name. A Folder with / at the end can not be an archive"))?;
                    let archive_path =
                        Config::get_or_create_archives_dir()?.join(Path::new(file_name));

                    archive_unpack(&archive_path, &unpack_path, &package_name)?;
                }
                bundle::SourceNode::Git(git_src) => {
                    let file_name = format!("{}.tar.gz", git_src.get_repo_prefix());
                    let archive_path =
                        Config::get_or_create_archives_dir()?.join(Path::new(&file_name));
                    archive_unpack(&archive_path, &unpack_path, &package_name)?;
                }
                bundle::SourceNode::File(file) => {
                    let src_path = file.get_bundle_path(bundle_path);
                    let final_path = unpack_path.join(file.get_target_path());
                    std::fs::copy(src_path, final_path).into_diagnostic()?;
                }
                bundle::SourceNode::Patch(patch) => {
                    let src_path = patch
                        .get_bundle_path(bundle_path)
                        .to_string_lossy()
                        .to_string();
                    let unpack_arg = unpack_path.to_string_lossy().to_string();
                    let mut patch_cmd = Command::new("gpatch");
                    patch_cmd.arg("-d");
                    patch_cmd.arg(&unpack_arg);
                    if let Some(drop_directories) = patch.drop_directories {
                        let strip_arg = format!("-p{}", drop_directories);
                        patch_cmd.arg(&strip_arg);
                    }
                    patch_cmd.arg("-i");
                    patch_cmd.arg(&src_path);

                    let status = patch_cmd.status().into_diagnostic()?;

                    if !status.success() {
                        return Err(miette::miette!("failed to patch sources"));
                    }
                }
                bundle::SourceNode::Overlay(overlay) => {
                    let src_path = overlay.get_bundle_path(bundle_path);
                    let final_path = unpack_path.clone();
                    let mut copy_opts = fs_extra::dir::CopyOptions::new();
                    copy_opts.overwrite = true;
                    copy_opts.content_only = true;
                    fs_extra::copy_items(&[src_path], final_path, &copy_opts).into_diagnostic()?;
                }
            }
        }
    }
    Ok(())
}

fn archive_unpack<P: AsRef<Path>>(local_file: P, final_path: P, name: &str) -> Result<()> {
    let local_file = local_file.as_ref();
    let final_path = final_path.as_ref();
    if !local_file.exists() {
        return Err(miette::miette!(format!(
            "archive path {} does not exist cannot unpack",
            local_file.display()
        )));
    }

    if final_path.exists() {
        println!("Archive for {} already extracted skipping", name);
        return Ok(());
    }

    let tmp_dir_path = Path::new("tmp.unpack");
    if !tmp_dir_path.exists() {
        DirBuilder::new().create(tmp_dir_path).into_diagnostic()?;
    } else {
        std::fs::remove_dir_all(&tmp_dir_path).into_diagnostic()?;
    }

    use compress_tools::*;
    use std::fs::File;

    let archive_file = File::open(local_file).into_diagnostic()?;

    uncompress_archive(archive_file, tmp_dir_path, Ownership::Ignore).into_diagnostic()?;

    let extracted_dirs = read_dir(tmp_dir_path)
        .into_diagnostic()?
        .into_iter()
        .filter_map(|e| match e {
            Ok(e) => Some(e.path()),
            Err(_) => None,
        })
        .collect::<Vec<PathBuf>>();
    let extracted_dir = extracted_dirs
        .first()
        .ok_or(miette::miette!("no directories extracted"))?;

    std::fs::rename(&extracted_dir, final_path).into_diagnostic()?;

    std::fs::remove_dir(tmp_dir_path).into_diagnostic()?;

    Ok(())
}
