use std::process::Command;

use crate::workspace::{DownloadFile, Workspace};
use bundle::Package;
use curl::easy::Easy2;
use miette::{IntoDiagnostic, Result, WrapErr};

pub(crate) fn download_and_verify(wks: &Workspace, pkg: &Package) -> Result<()> {
    for section in &pkg.sections {
        match section {
            bundle::Section::Sources(section) => {
                for src in &section.sources {
                    match src {
                        bundle::SourceNode::Archive(archive) => {
                            let url: url::Url = archive
                                .src
                                .parse()
                                .into_diagnostic()
                                .wrap_err("could not parse archive src argument as url")?;

                            let local_file = wks.get_file_path(url.clone())?;
                            if !local_file.exists() {
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
                                    continue;
                                } else {
                                    return Err(miette::miette!(format!("checksum missmatch for archive {}, expected: {}, actual {}", url.to_string(), archive.sha512, downloaded_file_hash)));
                                }
                            } else {
                                println!("File {} exists skipping", local_file.display());
                            }
                        }
                        bundle::SourceNode::Git(git) => {
                            if git.archive.is_some() {
                                git_archive_get(wks, &git)?;
                            } else {
                                git_clone_get(wks, &git)?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn git_clone_get(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let mut git_cmd = Command::new("git");
    let repo_prefix = git
        .repository
        .rsplit_once('/')
        .unwrap_or(("", &git.repository))
        .1;
    let repo_prefix_part = if let Some(split_sucess) = repo_prefix.split_once('.') {
        split_sucess.0.to_string()
    } else {
        repo_prefix.to_string()
    };

    let repo_prefix = if let Some(tag) = &git.tag {
        format!("{}-{}", repo_prefix_part, tag)
    } else if let Some(branch) = &git.branch {
        format!("{}-{}", repo_prefix_part, branch)
    } else {
        format!("{}-main", repo_prefix_part)
    };

    git_cmd.current_dir(&wks.get_archive_dir());
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
        Ok(())
    } else {
        Err(miette::miette!(format!(
            "Could not git clone {}",
            git.repository
        )))
    }
}

fn git_archive_get(wks: &Workspace, git: &bundle::GitSource) -> Result<()> {
    let mut git_cmd = Command::new("git");
    let repo_prefix = git
        .repository
        .rsplit_once('/')
        .unwrap_or(("", &git.repository))
        .1;
    let repo_prefix_part = if let Some(split_sucess) = repo_prefix.split_once('.') {
        split_sucess.0.to_string()
    } else {
        repo_prefix.to_string()
    };

    let repo_prefix = if let Some(tag) = &git.tag {
        format!("{}-{}", repo_prefix_part, tag)
    } else if let Some(branch) = &git.branch {
        format!("{}-{}", repo_prefix_part, branch)
    } else {
        format!("{}-main", repo_prefix_part)
    };

    let prefix_arg = format!("--prefix={}", &repo_prefix);
    let output_arg = format!("--output={}.tar.gz", &repo_prefix);
    let remote_arg = format!("--remote={}", &git.repository);

    git_cmd.current_dir(&wks.get_archive_dir());
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
        git_cmd.arg("main");
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
