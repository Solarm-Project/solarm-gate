use std::{collections::HashMap, process::Stdio};

use bundle::Bundle;
use miette::{IntoDiagnostic, Result};

use crate::{config::Settings, derive_source_name, workspace::Workspace};
use std::process::Command;

enum BuildTool {
    Make,
    Ninja,
}

impl ToString for BuildTool {
    fn to_string(&self) -> String {
        match self {
            BuildTool::Make => String::from("make"),
            BuildTool::Ninja => String::from("ninja"),
        }
    }
}

//TODO: custom install section
pub fn run_install(wks: &Workspace, pkg: &Bundle, settings: &Settings) -> Result<()> {
    let build_dir = wks.get_or_create_build_dir()?;
    let unpack_name = derive_source_name(
        pkg.package_document.name.clone(),
        &pkg.package_document.sources[0],
    );
    let unpack_path = build_dir.join(&unpack_name);
    if pkg.package_document.seperate_build_dir {
        let out_dir = build_dir.join("out");
        std::env::set_current_dir(&out_dir).into_diagnostic()?;
    } else {
        std::env::set_current_dir(&unpack_path).into_diagnostic()?;
    }

    let build_tool_check_dir = if pkg.package_document.seperate_build_dir {
        build_dir.join("out")
    } else {
        unpack_path.clone()
    };

    let build_tool = if build_tool_check_dir.join("Makefile").exists() {
        BuildTool::Make
    } else if build_tool_check_dir.join("build.ninja").exists() {
        BuildTool::Ninja
    } else {
        return Err(miette::miette!("no supported build tool could be detected make sure a Makefile or build.ninja file exists in the build directory"));
    };

    let mut env_flags: HashMap<String, String> = HashMap::new();
    env_flags.insert("PATH".into(), settings.get_search_path().join(":"));

    let proto_dir_path = wks.get_or_create_prototype_dir()?;
    let proto_dir_str = proto_dir_path.to_string_lossy().to_string();

    env_flags.insert(String::from("DESTDIR"), proto_dir_str.clone());
    let destdir_arg = format!("DESTDIR={}", &proto_dir_str);

    let mut build_cmd = Command::new(build_tool.to_string());
    build_cmd.env_clear();
    build_cmd.arg("install");
    build_cmd.envs(&env_flags);
    build_cmd.arg(&destdir_arg);

    build_cmd.stdin(Stdio::null());
    build_cmd.stdout(Stdio::inherit());

    println!(
        "Running {} install; into DESTDIR={}; env=[{}]",
        //option_vec.join(" "),
        build_tool.to_string(),
        &proto_dir_str,
        env_flags
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join(",")
    );

    let status = build_cmd.status().into_diagnostic()?;
    if status.success() {
        println!("Successfully installed {}", pkg.get_name());
    } else {
        return Err(miette::miette!(format!(
            "Could not build {}",
            pkg.get_name()
        )));
    }

    Ok(())
}
