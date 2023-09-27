use std::{
    fs::File,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{config::Settings, derive_source_name, workspace::Workspace};
use bundle::{Bundle, SourceNode};
use fs_extra::file::write_all;
use gate::Gate;
use microtemplate::{render, Substitutions};
use miette::{IntoDiagnostic, Result};

const DEFAULT_IPS_TEMPLATE: &str = r#"
#
# This file and its contents are supplied under the terms of the
# Common Development and Distribution License ("CDDL"), version 1.0.
# You may only use this file in accordance with the terms of version
# 1.0 of the CDDL.
#
# A full copy of the text of the CDDL should have accompanied this
# source.  A copy of the CDDL is also available via the Internet at
# http://www.illumos.org/license/CDDL.
#

#
# Copyright 2023 OpenFlowLabs
#

set name=pkg.fmri value=pkg:/{name}@{version},{build_version}-{branch_version}.{revision}
set name=pkg.summary value="{summary}"
set name=info.classification value="org.opensolaris.category.2008:{classification}"
set name=info.upstream-url value="{project_url}"
set name=info.source-url value="{source_url}"

license {license_file_name} license='{license_name}'

"#;
//TODO remove drop dir transform here and put it into standard transforms
//TODO implement ips component version formatter. build_num (year)

#[derive(Substitutions)]
struct StringInterpolationVars<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub build_version: &'a str,
    pub branch_version: &'a str,
    pub revision: &'a str,
    pub summary: &'a str,
    pub classification: &'a str,
    pub project_url: &'a str,
    pub source_url: &'a str,
    pub license_file_name: &'a str,
    pub license_name: &'a str,
}

fn get_source_url<'a>(src: &'a SourceNode) -> &'a str {
    match src {
        SourceNode::Archive(a) => &a.src,
        SourceNode::Git(g) => &g.repository,
        _ => "",
    }
}

pub fn run_generate_filelist(wks: &Workspace, pkg: &Bundle) -> Result<()> {
    let proto_path = wks.get_or_create_prototype_dir()?;
    let manifest_path = wks.get_or_create_manifest_dir()?;

    let formatted_manifest = File::create(manifest_path.join("filelist.fmt")).into_diagnostic()?;

    let pkg_send_cmd = Command::new("pkgsend")
        .arg("generate")
        .arg(proto_path.to_string_lossy().to_string())
        .stdout(Stdio::piped())
        .spawn()
        .into_diagnostic()?;

    let pkg_fmt_cmd_status = Command::new("pkgfmt")
        .stdin(pkg_send_cmd.stdout.unwrap())
        .stdout(formatted_manifest)
        .status()
        .into_diagnostic()?;

    if pkg_fmt_cmd_status.success() {
        println!("Generated filelist for {}", pkg.get_name());
        Ok(())
    } else {
        Err(miette::miette!("non zero code returned from pkgfmt"))
    }
}
pub fn run_mogrify(
    wks: &Workspace,
    pkg: &Bundle,
    gate: Option<Gate>,
    transform_includes: Option<PathBuf>,
) -> Result<()> {
    let vars = StringInterpolationVars {
        name: &pkg.get_name(),
        version: &pkg
            .package_document
            .version
            .clone()
            .unwrap_or(String::from("0.5.11")), //TODO take this default version from the gate
        build_version: &gate.clone().unwrap_or(Gate::default()).version,
        branch_version: &gate.clone().unwrap_or(Gate::default()).branch,
        revision: &pkg
            .package_document
            .revision
            .clone()
            .unwrap_or(String::from("1")),
        summary: &pkg
            .package_document
            .summary
            .clone()
            .ok_or(miette::miette!("no summary specified"))?,
        classification: &pkg
            .package_document
            .classification
            .clone()
            .ok_or(miette::miette!("no classification specified"))?,
        project_url: &pkg
            .package_document
            .project_url
            .clone()
            .ok_or(miette::miette!("no project_url specified"))?,
        source_url: get_source_url(&pkg.package_document.sources[0].sources[0]),
        license_file_name: &pkg
            .package_document
            .license_file
            .clone()
            .ok_or(miette::miette!("no license_file specified"))?,
        license_name: &pkg
            .package_document
            .license
            .clone()
            .ok_or(miette::miette!("no license specified"))?,
    };

    let manifest_path = wks.get_or_create_manifest_dir()?;

    let manifest = render(DEFAULT_IPS_TEMPLATE, vars);

    let mogrified_manifest = File::create(manifest_path.join("mogrified.mog")).into_diagnostic()?;

    write_all(manifest_path.join("generated.p5m"), &manifest).into_diagnostic()?;

    let include_path = if let Some(gate) = gate {
        if !gate.default_transforms.is_empty() {
            let mut include_str = gate
                .default_transforms
                .into_iter()
                .map(|tr| tr.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            include_str.push_str("\n");
            let inc_path = manifest_path.join("includes.mog");
            println!("Adding includes {} to includes.mog", &include_str);
            write_all(&inc_path, &include_str).into_diagnostic()?;
            Some(inc_path.to_string_lossy().to_string())
        } else {
            println!("Gate {} has no transforms", gate.name);
            None
        }
    } else {
        println!("Not building against a gate not adding gate transforms");
        None
    };

    let mut pkg_mogrify_cmd = Command::new("pkgmogrify");

    if let Some(includes_path) = transform_includes {
        pkg_mogrify_cmd.arg("-I").arg(&includes_path);
    }
    pkg_mogrify_cmd
        .current_dir("..")
        .arg(
            manifest_path
                .join("generated.p5m")
                .to_string_lossy()
                .to_string(),
        )
        .arg(
            manifest_path
                .join("filelist.fmt")
                .to_string_lossy()
                .to_string(),
        );

    if let Some(includes) = include_path {
        pkg_mogrify_cmd.arg(&includes);
    }

    if let Some(mog_file_path) = pkg.get_mogrify_manifest() {
        pkg_mogrify_cmd.arg(&mog_file_path.to_string_lossy().to_string());
    }

    pkg_mogrify_cmd.stdout(Stdio::piped());
    let pkg_mogrify_status = pkg_mogrify_cmd.spawn().into_diagnostic()?;

    let pkg_fmt_cmd_status = Command::new("pkgfmt")
        .stdin(pkg_mogrify_status.stdout.unwrap())
        .stdout(mogrified_manifest)
        .status()
        .into_diagnostic()?;

    if pkg_fmt_cmd_status.success() {
        println!("Mogrified manifests for {}", pkg.get_name());
        Ok(())
    } else {
        Err(miette::miette!("non zero code returned from pkgfmt"))
    }
}

pub fn run_generate_pkgdepend(wks: &Workspace, pkg: &Bundle) -> Result<()> {
    let manifest_path = wks.get_or_create_manifest_dir()?;
    let prototype_path = wks.get_or_create_prototype_dir()?;

    let depend_manifest = File::create(manifest_path.join("generated.dep")).into_diagnostic()?;

    let pkg_depend_cmd = Command::new("pkgdepend")
        .arg("generate")
        .arg("-m")
        .arg("-d")
        .arg(prototype_path.to_string_lossy().to_string())
        .arg(
            manifest_path
                .join("mogrified.mog")
                .to_string_lossy()
                .to_string(),
        )
        .stdout(Stdio::piped())
        .spawn()
        .into_diagnostic()?;

    let pkg_fmt_cmd_status = Command::new("pkgfmt")
        .stdin(pkg_depend_cmd.stdout.unwrap())
        .stdout(depend_manifest)
        .status()
        .into_diagnostic()?;

    if pkg_fmt_cmd_status.success() {
        println!("Generated dependency entries for {}", pkg.get_name());
        Ok(())
    } else {
        Err(miette::miette!("non zero code returned from pkgfmt"))
    }
}

pub fn run_resolve_dependencies(wks: &Workspace, pkg: &Bundle) -> Result<()> {
    let manifest_path = wks.get_or_create_manifest_dir()?;

    let pkg_depend_cmd = Command::new("pkgdepend")
        .arg("resolve")
        .arg("-m")
        .arg(
            manifest_path
                .join("generated.dep")
                .to_string_lossy()
                .to_string(),
        )
        .stdout(Stdio::inherit())
        .status()
        .into_diagnostic()?;

    if pkg_depend_cmd.success() {
        println!("Resolved dependencies for {}", pkg.get_name());
        Ok(())
    } else {
        Err(miette::miette!("non zero code returned from pkgfmt"))
    }
}

pub fn run_lint(wks: &Workspace, pkg: &Bundle) -> Result<()> {
    let manifest_path = wks.get_or_create_manifest_dir()?;

    let pkg_lint_cmd = Command::new("pkglint")
        .arg(
            manifest_path
                .join("generated.dep.res")
                .to_string_lossy()
                .to_string(),
        )
        .stdout(Stdio::inherit())
        .status()
        .into_diagnostic()?;

    if pkg_lint_cmd.success() {
        println!("Lint success for {}", pkg.get_name());
        Ok(())
    } else {
        Err(miette::miette!("non zero code returned from pkglint"))
    }
}

pub fn ensure_repo_with_publisher_exists(publisher: &str) -> Result<()> {
    let repo_base = Settings::get_or_create_repo_dir()?;

    if !repo_base.join("pkg5.repository").exists() {
        let pkg_repo_status = Command::new("pkgrepo")
            .arg("create")
            .arg(&repo_base.to_string_lossy().to_string())
            .stdout(Stdio::inherit())
            .status()
            .into_diagnostic()?;
        if !pkg_repo_status.success() {
            return Err(miette::miette!(
                "pkgrepo create failed with non zero exit code"
            ));
        }
    }

    if !repo_base.join("publisher").join(publisher).exists() {
        let pkg_repo_status = Command::new("pkgrepo")
            .arg("add-publisher")
            .arg("-s")
            .arg(&repo_base.to_string_lossy().to_string())
            .arg(publisher)
            .stdout(Stdio::inherit())
            .status()
            .into_diagnostic()?;
        if !pkg_repo_status.success() {
            return Err(miette::miette!(
                "pkgrepo create failed with non zero exit code"
            ));
        }
    }

    Ok(())
}

pub fn publish_package(wks: &Workspace, pkg: &Bundle, publisher: &str) -> Result<()> {
    let proto_dir = wks.get_or_create_prototype_dir()?;
    let build_dir = wks.get_or_create_build_dir()?;
    let unpack_name = derive_source_name(
        pkg.package_document.name.clone(),
        &pkg.package_document.sources[0],
    );
    let unpack_path = build_dir.join(&unpack_name);
    let repo_path = Settings::get_or_create_repo_dir()?;
    let manifest = wks.get_or_create_manifest_dir()?.join("generated.dep.res");
    let pkgsend_status = Command::new("pkgsend")
        .arg("publish")
        .arg("-d")
        .arg(&proto_dir.to_string_lossy().to_string())
        .arg("-d")
        .arg(&unpack_path.to_string_lossy().to_string())
        .arg("-s")
        .arg(&repo_path.to_string_lossy().to_string())
        .arg(&manifest.to_string_lossy().to_string())
        .stdout(Stdio::inherit())
        .status()
        .into_diagnostic()?;

    if pkgsend_status.success() {
        println!("Package {} built and published sucessfully", pkg.get_name());
        println!(
            "Install with pkg set-publisher {}; pkg install -g {} {}",
            publisher,
            repo_path.display(),
            pkg.get_name()
        );
        Ok(())
    } else {
        Err(miette::miette!(
            "non zero code returned from pkgsend publish"
        ))
    }
}
