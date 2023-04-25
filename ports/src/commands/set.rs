use std::path::PathBuf;

use bundle::Bundle;
use clap::Subcommand;
use miette::Result;

use crate::workspace::Workspace;

#[derive(Debug, Subcommand)]
pub enum Sections {
    Maintainer {
        name: String,
    },
    Classification {
        name: String,
    },
    Summary {
        name: String,
    },
    License {
        spdx_id: Option<String>,
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    Prefix {
        prefix: String,
    },
    Version {
        version: String,
    },
    Revision {
        revision: String,
    },
    ProjectURL {
        project_url: String,
    },
}

enum LicenseAction {
    SetID(String),
    SetIDAndFile(String, PathBuf),
    GetIDFromFile(PathBuf),
    Bad,
}

fn detect_license_action(id: Option<String>, file: Option<PathBuf>) -> LicenseAction {
    if let Some(file) = file {
        if let Some(id) = id {
            LicenseAction::SetIDAndFile(id, file)
        } else {
            LicenseAction::GetIDFromFile(file)
        }
    } else {
        if let Some(id) = id {
            LicenseAction::SetID(id)
        } else {
            LicenseAction::Bad
        }
    }
}

pub fn handle_set(_wks: &Workspace, section: &Sections, pkg: &mut Bundle) -> Result<()> {
    match section {
        Sections::Maintainer { name } => {
            pkg.package_document.maintainer = Some(name.clone());
            Ok(())
        }
        Sections::Classification { name } => {
            pkg.package_document.classification = Some(name.clone());
            Ok(())
        }
        Sections::Summary { name } => {
            pkg.package_document.summary = Some(name.clone());
            Ok(())
        }
        Sections::License { spdx_id, file } => {
            match detect_license_action(spdx_id.clone(), file.clone()) {
                LicenseAction::SetID(id) => {
                    pkg.package_document.license = Some(id);
                    Ok(())
                }
                LicenseAction::SetIDAndFile(id, file) => {
                    pkg.package_document.license = Some(id);
                    pkg.package_document.license_file = Some(file.to_string_lossy().to_string());
                    Ok(())
                }
                LicenseAction::GetIDFromFile(_) => Err(miette::miette!("Reading the license file for the correct identifier is currently not supported please set both the license string and the file where the license is located")),
                LicenseAction::Bad => Err(miette::miette!(
                    "either file or license identifier must be set"
                )),
            }
        }
        Sections::Prefix { prefix } => {
            pkg.package_document.prefix = Some(prefix.clone());
            Ok(())
        }
        Sections::Version { version } => {
            pkg.package_document.version = Some(version.clone());
            pkg.package_document.revision = None;
            Ok(())
        }
        Sections::Revision { revision } => {
            pkg.package_document.revision = Some(revision.clone());
            Ok(())
        }
        Sections::ProjectURL { project_url } => {
            pkg.package_document.project_url = Some(project_url.clone());
            Ok(())
        }
    }
}
