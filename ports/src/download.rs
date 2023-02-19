use std::{fs, path::Path, process::Command};

use crate::{
    config::Config,
    path::add_extension,
    workspace::{DownloadFile, Workspace},
};
use bundle::SourceSection;
use curl::easy::Easy2;
use miette::{IntoDiagnostic, Result, WrapErr};

pub(crate) fn download_and_verify(
    wks: &Workspace,
    src_sections: &[SourceSection],
    archive_clean: bool,
) -> Result<()> {
    for section in src_sections {
        for src in &section.sources {
            match src {
                bundle::SourceNode::Archive(archive) => {
                    let url: url::Url = archive
                        .src
                        .parse()
                        .into_diagnostic()
                        .wrap_err("could not parse archive src argument as url")?;

                    let local_file = wks.get_file_path(url.clone())?;

                    let file_name = local_file.file_name().ok_or(miette::miette!("Archive must have a file_name. A Folder with / at the end can not be an archive"))?;
                    let archive_path =
                        Config::get_or_create_archives_dir()?.join(Path::new(file_name));

                    if archive_clean {
                        std::fs::remove_file(&archive_path).ok();
                    }

                    if !archive_path.exists() {
                        println!("Downloading {}", url.to_string());
                        let mut easy = Easy2::new(wks.open_local_file(url.clone())?);
                        easy.get(true).into_diagnostic()?;
                        easy.url(&url.to_string()).into_diagnostic()?;
                        easy.progress(true).into_diagnostic()?;
                        easy.perform().into_diagnostic()?;
                        let local_file = { easy.get_mut() as &mut DownloadFile };
                        let downloaded_file_hash = local_file.get_hash();
                        if downloaded_file_hash == archive.sha512 {
                            println!("Success, checksums match");
                            let local_path = local_file.get_path();
                            drop(local_file);
                            fs::copy(&local_path, archive_path).into_diagnostic()?;
                            fs::remove_file(&local_path).into_diagnostic()?;
                        } else {
                            return Err(miette::miette!(format!(
                                "checksum missmatch for archive {}, expected: {}, actual {}",
                                url.to_string(),
                                archive.sha512,
                                downloaded_file_hash
                            )));
                        }
                    } else {
                        println!("File {} exists skipping", local_file.display());
                    }
                }
                bundle::SourceNode::Git(git) => {
                    let git_prefix = &git.get_repo_prefix();
                    let git_repo_path = &wks.get_or_create_download_dir()?.join(&git_prefix);
                    let archive_path = add_extension(
                        Config::get_or_create_archives_dir()?.join(&git_prefix),
                        "tar.gz",
                    );

                    if archive_clean {
                        std::fs::remove_file(&archive_path).ok();
                    }

                    if !archive_path.exists() {
                        if !git_repo_path.exists() {
                            if git.archive.is_some() {
                                git_archive_get(wks, &git)?;
                            } else {
                                git_clone_get(wks, &git)?;
                            }
                        } else {
                            if git.must_stay_as_repo.is_some() {
                                println!("Creating Archive of full repo");
                                make_git_archive_with_tar(wks, git)?;
                            } else {
                                println!("Creating git-archive based archive from git");
                                make_git_archive(wks, git)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn git_clone_get(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let mut git_cmd = Command::new("git");

    let repo_prefix = git.get_repo_prefix();

    git_cmd.current_dir(&wks.get_or_create_download_dir()?);
    git_cmd.arg("clone");
    git_cmd.arg("--single-branch");
    if let Some(tag) = &git.tag {
        git_cmd.arg("--branch");
        git_cmd.arg(tag);
    } else if let Some(branch) = &git.branch {
        git_cmd.arg("--branch");
        git_cmd.arg(branch);
    }
    git_cmd.arg(&git.repository);
    git_cmd.arg(&repo_prefix);

    let status = git_cmd.status().into_diagnostic()?;
    if status.success() {
        println!("Git successfully cloned from remote");
    } else {
        return Err(miette::miette!(format!(
            "Could not git clone {}",
            git.repository
        )));
    }

    if git.must_stay_as_repo.is_some() {
        println!("Creating Archive of full repo");
        make_git_archive_with_tar(wks, git)
    } else {
        println!("Creating git-archive based archive from git");
        make_git_archive(wks, git)
    }
}

fn make_git_archive_with_tar(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let repo_prefix = git.get_repo_prefix();

    let mut archive_cmd = Command::new("gtar");
    archive_cmd.current_dir(&wks.get_or_create_download_dir()?);
    archive_cmd.arg("-czf");
    let archive_name_arg = add_extension(
        Config::get_or_create_archives_dir()?.join(&repo_prefix),
        "tar.gz",
    )
    .to_string_lossy()
    .to_string();
    archive_cmd.arg(&archive_name_arg);
    archive_cmd.arg(&repo_prefix);

    let status = archive_cmd.status().into_diagnostic()?;
    if status.success() {
        println!(
            "Git Archive {}.tar.gz successfully created by way of tar",
            &repo_prefix
        );
        Ok(())
    } else {
        Err(miette::miette!(format!(
            "Could not create archive of {}",
            &repo_prefix
        )))
    }
}

fn make_git_archive(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let repo_prefix = git.get_repo_prefix();

    let mut archive_cmd = Command::new("git");
    archive_cmd.current_dir(&wks.get_or_create_download_dir()?.join(&repo_prefix));
    archive_cmd.arg("archive");
    archive_cmd.arg("--format=tar.gz");
    let prefix_arg = format!("--prefix={}/", &repo_prefix);
    let output_arg = format!(
        "--output={}",
        add_extension(
            Config::get_or_create_archives_dir()?.join(&repo_prefix),
            "tar.gz"
        )
        .to_string_lossy()
        .to_string()
    );
    archive_cmd.arg(&prefix_arg);
    archive_cmd.arg(&output_arg);
    archive_cmd.arg("HEAD");

    let status = archive_cmd.status().into_diagnostic()?;
    if status.success() {
        println!("Git Archive {}.tar.gz successfully created", &repo_prefix);
        Ok(())
    } else {
        Err(miette::miette!(format!(
            "Could not create archive of {}",
            &repo_prefix
        )))
    }
}

fn git_archive_get(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let mut git_cmd = Command::new("git");
    let repo_prefix = git.get_repo_prefix();

    let prefix_arg = format!("--prefix={}", &repo_prefix);
    let output_arg = format!(
        "--output={}",
        add_extension(
            Config::get_or_create_archives_dir()?.join(&repo_prefix),
            "tar.gz"
        )
        .to_string_lossy()
        .to_string()
    );
    let remote_arg = format!("--remote={}", &git.repository);

    git_cmd.current_dir(&wks.get_or_create_download_dir()?);
    git_cmd.arg("archive");
    git_cmd.arg("--format=tar.gz");
    git_cmd.arg(prefix_arg);
    git_cmd.arg(output_arg);
    git_cmd.arg(remote_arg);
    git_cmd.arg("-v");
    if let Some(tag) = &git.tag {
        git_cmd.arg(tag);
    } else if let Some(branch) = &git.branch {
        git_cmd.arg(branch);
    } else {
        git_cmd.arg("HEAD");
    }

    let status = git_cmd.status().into_diagnostic()?;
    if status.success() {
        println!("Archive sucesscully copied from git remote");
        Ok(())
    } else {
        Err(miette::miette!(format!(
            "Could not get git archive for {}",
            git.repository
        )))
    }
}
