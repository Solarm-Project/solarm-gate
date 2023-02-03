use std::{
    collections::HashMap,
    fs::DirBuilder,
    process::{Command, Stdio},
};

use bundle::{Bundle, ConfigureBuildSection, ScriptBuildSection};
use miette::{IntoDiagnostic, Result, WrapErr};

use crate::{derive_source_name, workspace::Workspace};

pub fn build_package_sources(wks: &Workspace, pkg: &Bundle) -> Result<()> {
    match pkg.package_document.ensure_build_section() {
        bundle::BuildSection::Configure(c) => build_using_automake(wks, pkg, &c),
        bundle::BuildSection::CMake => todo!(),
        bundle::BuildSection::Meson => todo!(),
        bundle::BuildSection::Build(s) => {
            build_using_scripts(wks, pkg, &s)?;

            Ok(())
        }
        bundle::BuildSection::NoBuild => todo!(),
    }?;

    Ok(())
}

fn build_using_scripts(
    wks: &Workspace,
    pkg: &Bundle,
    build_section: &ScriptBuildSection,
) -> Result<()> {
    let build_dir = wks.get_or_create_build_dir()?;
    let unpack_name = derive_source_name(
        pkg.package_document.name.clone(),
        &pkg.package_document.sources[0],
    );
    let unpack_path = build_dir.join(&unpack_name);
    std::env::set_current_dir(&unpack_path).into_diagnostic()?;

    for script in &build_section.scripts {
        let status = Command::new(pkg.get_path().join(&script.name))
            .stdout(Stdio::inherit())
            .status()
            .into_diagnostic()?;

        if status.success() {
            println!(
                "Successfully ran script {} in package {}",
                script.name,
                pkg.get_name()
            );
        } else {
            return Err(miette::miette!(format!(
                "Could not run script {} in package {}",
                script.name,
                pkg.get_name()
            )));
        }

        println!(
            "Copying prototype directory {} to workspace prototype directory",
            &script.prototype_dir.display()
        );

        let mut copy_options = fs_extra::dir::CopyOptions::default();
        copy_options.overwrite = true;
        copy_options.content_only = true;

        if let Some(prefix) = &pkg.package_document.prefix {
            let prefix = if prefix.starts_with("/") {
                &prefix[1..]
            } else {
                prefix.as_str()
            };

            let target_path = wks.get_or_create_prototype_dir()?.join(prefix);
            if !target_path.exists() {
                DirBuilder::new()
                    .recursive(true)
                    .create(&target_path)
                    .into_diagnostic()?;
                println!("Creating target path {}", target_path.display());
            }

            fs_extra::dir::copy(
                unpack_path.join(&script.prototype_dir),
                &target_path,
                &copy_options,
            )
            .into_diagnostic()?;
        } else {
            fs_extra::dir::copy(
                unpack_path.join(&script.prototype_dir),
                wks.get_or_create_prototype_dir()?,
                &copy_options,
            )
            .into_diagnostic()?;
        }
    }

    for package_directory in &build_section.package_directories {
        let target_path = if let Some(prefix) = &pkg.package_document.prefix {
            let prefix = if prefix.starts_with("/") {
                &prefix[1..]
            } else {
                prefix.as_str()
            };

            wks.get_or_create_prototype_dir()?
                .join(&prefix)
                .join(&package_directory.target)
        } else {
            wks.get_or_create_prototype_dir()?
                .join(&package_directory.target)
        };
        println!("Copying directory to prototype dir");
        println!("Target Path: {}", target_path.display());
        let directory_full_from_path = unpack_path.join(&package_directory.src);
        println!("Source Path: {}", directory_full_from_path.display());
        if !target_path.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&target_path)
                .into_diagnostic()?;
            println!("Creating target dir");
        }
        let mut copy_options = fs_extra::dir::CopyOptions::default();
        copy_options.overwrite = true;
        copy_options.content_only = true;
        fs_extra::dir::copy(directory_full_from_path, target_path, &copy_options)
            .into_diagnostic()?;
        println!("Copy suceeded");
    }

    println!("Build for package {} finished", pkg.get_name());

    Ok(())
}

fn build_using_automake(
    wks: &Workspace,
    pkg: &Bundle,
    build_section: &ConfigureBuildSection,
) -> Result<()> {
    let dotenv_env: Vec<(String, String)> =
        crate::env::get_environment(pkg.get_path().parent().unwrap())?;
    let build_dir = wks.get_or_create_build_dir()?;
    let unpack_name = derive_source_name(
        pkg.package_document.name.clone(),
        &pkg.package_document.sources[0],
    );
    let unpack_path = build_dir.join(&unpack_name);
    if pkg.package_document.seperate_build_dir {
        let out_dir = build_dir.join("out");
        DirBuilder::new().create(&out_dir).into_diagnostic()?;
        std::env::set_current_dir(&out_dir).into_diagnostic()?;
    } else {
        std::env::set_current_dir(&unpack_path).into_diagnostic()?;
    }

    let mut option_vec: Vec<_> = vec![];
    let mut env_flags: HashMap<String, String> = HashMap::new();

    for option in build_section.options.iter() {
        let opt_arg = format!("--{}", option.option);
        option_vec.push(opt_arg);
    }

    for flag in build_section.flags.iter() {
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

    if let Some(prefix) = &pkg.package_document.prefix {
        option_vec.push(format!("--prefix={}", prefix));
    }

    for (env_key, env_var) in dotenv_env {
        env_flags.insert(env_key, env_var);
    }

    let proto_dir_path = wks.get_or_create_prototype_dir()?;
    let proto_dir_str = proto_dir_path.to_string_lossy().to_string();

    env_flags.insert(String::from("DESTDIR"), proto_dir_str.clone());
    let destdir_arg = format!("DESTDIR={}", &proto_dir_str);

    let bin_path = if pkg.package_document.seperate_build_dir {
        unpack_path.join("configure").to_string_lossy().to_string()
    } else {
        String::from("./configure")
    };

    let mut configure_cmd = Command::new(&bin_path);
    configure_cmd.env_clear();
    configure_cmd.envs(&env_flags);
    configure_cmd.args(&option_vec);
    configure_cmd.arg(&destdir_arg);

    configure_cmd.stdin(Stdio::null());
    configure_cmd.stdout(Stdio::inherit());

    println!(
        "Running configure with options {}; {}; env=[{}]",
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

    crate::compile::run_compile(wks, pkg).wrap_err("compilation step failed")?;

    crate::install::run_install(wks, pkg).wrap_err("installation step failed")
}
