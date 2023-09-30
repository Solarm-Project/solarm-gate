use std::process::{Command, Stdio};

use crate::workspace::Workspace;
use bundle::Bundle;
use miette::IntoDiagnostic;

fn derive_output_name(pkg: &Bundle) -> String {
    if let Some(version) = &pkg.package_document.version {
        format!("{}-{}.tar.gz", pkg.get_name().replace("/", "_"), version)
    } else {
        format!("{}.tar.gz", pkg.get_name().replace("/", "_"))
    }
}

pub fn make_release_tarball(wks: &Workspace, pkg: &Bundle) -> miette::Result<()> {
    let proto_dir = wks.get_or_create_prototype_dir()?;
    let output_dir = crate::config::Settings::get_or_create_output_dir()?;
    let tarball_path_string = output_dir
        .join(derive_output_name(pkg))
        .to_string_lossy()
        .to_string();

    let dirs = std::fs::read_dir(&proto_dir)
        .into_diagnostic()?
        .into_iter()
        .map(|p| {
            p.unwrap()
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<String>>();

    let mut tar_cmd = Command::new("gtar");
    tar_cmd.current_dir(&proto_dir);
    tar_cmd.arg("-czf");
    tar_cmd.arg(&tarball_path_string);
    tar_cmd.args(
        dirs.iter()
            .map(|p| p.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
    );
    tar_cmd.stdout(Stdio::inherit());
    let tar_cmd_status = tar_cmd.status().into_diagnostic()?;

    if tar_cmd_status.success() {
        println!("Generated Output tarball {}", tarball_path_string);
        Ok(())
    } else {
        Err(miette::miette!(
            "gtar returned error code check above for error"
        ))
    }
}
