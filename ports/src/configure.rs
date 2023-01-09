use std::{
    collections::HashMap,
    fs::DirBuilder,
    path::PathBuf,
    process::{Command, Stdio},
};

use bundle::{BuildSection, BuildType, Bundle};
use miette::{IntoDiagnostic, Result};

use crate::{derive_source_name, workspace::Workspace, CrossTriple};

pub fn configure_package_sources(
    wks: &Workspace,
    pkg: &Bundle,
    prefix: Option<PathBuf>,
    cross_tripple: Option<CrossTriple>,
) -> Result<()> {
    //TODO make into implementation that borrows with &str instead of moving with String
    let build_type: BuildType = pkg.package_document.build.build_type.clone().try_into()?;
    match build_type {
        BuildType::Configure => configure_using_automake(wks, pkg, prefix, cross_tripple),
        BuildType::CMake => todo!(),
        BuildType::Meson => todo!(),
    }
}

fn configure_using_automake(
    wks: &Workspace,
    pkg: &Bundle,
    prefix: Option<PathBuf>,
    cross_tripple: Option<CrossTriple>,
) -> Result<()> {
    /* ./configure \
        --with-sysroot \
        --target=aarch64-solaris2.11 \
        --prefix=$(CROSS) \
        --enable-initfini-array
    */

    let dotenv_env: Vec<(String, String)> =
        crate::env::get_environment(pkg.get_path().parent().unwrap())?;
    let build_dir = wks.get_or_create_build_dir()?;
    let unpack_name = derive_source_name(
        pkg.package_document.name.clone(),
        &pkg.package_document.sources[0],
    );
    let unpack_path = build_dir.join(&unpack_name);
    std::env::set_current_dir(&unpack_path).into_diagnostic()?;

    let mut option_vec: Vec<_> = vec![];
    let mut env_flags: HashMap<String, String> = HashMap::new();
    for option in pkg.package_document.build.options.iter() {
        let opt_arg = format!("--{}", option.option);
        option_vec.push(opt_arg);
    }

    for flag in pkg.package_document.build.flags.iter() {
        for flag_name in vec![
            String::from("CFLAGS"),
            String::from("CXXFLAGS"),
            String::from("CPPFLAGS"),
            String::from("FFLAGS"),
        ] {
            if env_flags.contains_key(&flag_name) {
                let flag_ref = env_flags.get_mut(&flag_name).unwrap();
                flag_ref.push_str(&flag.flag);
            } else {
                env_flags.insert(flag_name, flag.flag.clone());
            }
        }
    }

    if let Some(cross_tripple) = cross_tripple {
        let config_sections = pkg
            .package_document
            .build
            .cross_tool_options
            .iter()
            .filter(|s| s.build_type == cross_tripple.to_string())
            .map(|s| s.clone())
            .collect::<Vec<BuildSection>>();
        let build_section = &config_sections[0];
        for option in build_section.options.iter() {
            let opt_arg = format!("--{}", option.option);
            option_vec.push(opt_arg);
        }

        for flag in build_section.flags.iter() {
            for flag_name in vec![
                String::from("CFLAGS"),
                String::from("CPPFLAGS"),
                String::from("FFLAGS"),
            ] {
                if env_flags.contains_key(&flag_name) {
                    let flag_ref = env_flags.get_mut(&flag_name).unwrap();
                    flag_ref.push_str(&flag.flag);
                } else {
                    env_flags.insert(flag_name, flag.flag.clone());
                }
            }
        }
    }

    if let Some(prefix) = prefix {
        if !prefix.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&prefix)
                .into_diagnostic()?;
        }
        let prefix = std::fs::canonicalize(&prefix).into_diagnostic()?;
        option_vec.push(format!("--prefix={}", prefix.to_string_lossy().to_string()));
    } else if let Some(prefix) = pkg.package_document.prefix.clone() {
        option_vec.push(format!("--prefix={}", prefix));
    }

    for (env_key, env_var) in dotenv_env {
        env_flags.insert(env_key, env_var);
    }

    let proto_dir_path = wks.get_or_create_prototype_dir()?;
    let proto_dir_str = proto_dir_path.to_string_lossy().to_string();

    env_flags.insert(String::from("DESTDIR"), proto_dir_str.clone());
    let destdir_arg = format!("DESTDIR={}", &proto_dir_str);

    let mut configure_cmd = Command::new("./configure");
    configure_cmd.env_clear();
    configure_cmd.envs(&env_flags);
    configure_cmd.args(&option_vec);
    configure_cmd.arg(&destdir_arg);

    configure_cmd.stdin(Stdio::null());
    configure_cmd.stdout(Stdio::inherit());

    println!(
        "Running ./configure with options {}; {}; env=[{}]",
        option_vec.join(" "),
        destdir_arg,
        env_flags
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join(",")
    );

    let status = configure_cmd.status().into_diagnostic()?;
    if status.success() {
        println!("Successfully configured {}", pkg.get_name());
    } else {
        return Err(miette::miette!(format!(
            "Could not configure {}",
            pkg.get_name()
        )));
    }

    Ok(())
}
