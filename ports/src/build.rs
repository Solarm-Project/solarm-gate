use std::{
    collections::HashMap,
    fs::DirBuilder,
    io::Write,
    path::{Path, PathBuf},
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
            .env(
                "PROTO_DIR",
                wks.get_or_create_prototype_dir()
                    .into_diagnostic()?
                    .into_os_string(),
            )
            .env("UNPACK_DIR", &unpack_path.clone().into_os_string())
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

        if let Some(prototype_dir) = &script.prototype_dir {
            println!(
                "Copying prototype directory {} to workspace prototype directory",
                &prototype_dir.display()
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

                let src_path = unpack_path.join(&prototype_dir);

                println!("src: {}", &src_path.display());
                println!("exists?: {}", src_path.exists());
                println!("target: {}", &target_path.display());
                println!("exists?: {}", &target_path.exists());

                fs_extra::dir::copy(&src_path, &target_path, &copy_options).into_diagnostic()?;
            } else {
                fs_extra::dir::copy(
                    unpack_path.join(&prototype_dir),
                    wks.get_or_create_prototype_dir()?,
                    &copy_options,
                )
                .into_diagnostic()?;
            }
        }
    }

    for install_directive in &build_section.install_directives {
        let target_path = if let Some(prefix) = &pkg.package_document.prefix {
            let prefix = if prefix.starts_with("/") {
                &prefix[1..]
            } else {
                prefix.as_str()
            };

            wks.get_or_create_prototype_dir()?
                .join(&prefix)
                .join(&install_directive.target)
        } else {
            wks.get_or_create_prototype_dir()?
                .join(&install_directive.target)
        };
        println!("Copying directory to prototype dir");
        println!("Target Path: {}", target_path.display());
        let src_full_path = unpack_path.join(&install_directive.src);
        println!("Source Path: {}", src_full_path.display());
        if !target_path.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(&target_path)
                .into_diagnostic()?;
            println!("Creating target dir");
        }

        if let Some(pattern) = &install_directive.pattern {
            let mut copy_options = fs_extra::file::CopyOptions::default();
            copy_options.overwrite = true;
            let files = file_matcher::FilesNamed::regex(pattern)
                .within(&src_full_path)
                .find()
                .into_diagnostic()?;
            println!("Copying via rsync");
            copy_with_rsync(wks, &src_full_path, &target_path, files)?;
        } else if let Some(fmatch) = &install_directive.fmatch {
            let files = file_matcher::FilesNamed::wildmatch(fmatch)
                .within(&src_full_path)
                .find()
                .into_diagnostic()?;
            println!("Copying via rsync");
            copy_with_rsync(wks, &src_full_path, &target_path, files)?;
        } else {
            let mut copy_options = fs_extra::dir::CopyOptions::default();
            copy_options.overwrite = true;
            copy_options.content_only = true;
            fs_extra::dir::copy(src_full_path, target_path, &copy_options).into_diagnostic()?;
        }
        println!("Copy suceeded");
    }

    println!("Build for package {} finished", pkg.get_name());

    Ok(())
}

fn copy_with_rsync<P: AsRef<Path>>(
    wks: &Workspace,
    from: P,
    to: P,
    file_list: Vec<PathBuf>,
) -> Result<()> {
    //write file_list to known location into file
    let contents_file_path = wks.get_or_create_build_dir()?.join("install_file_list.txt");

    let src_path_string = from.as_ref().to_string_lossy().to_string();

    let file_list = file_list
        .into_iter()
        .map(|p| {
            p.to_string_lossy()
                .to_string()
                .replace(&src_path_string, "")
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("writing file list:\n{}", &file_list);

    let mut contents_file = std::fs::File::create(&contents_file_path).into_diagnostic()?;
    contents_file
        .write_all(&mut file_list.as_bytes())
        .into_diagnostic()?;
    drop(contents_file);
    let contents_file_arg = format!(
        "--files-from={}",
        contents_file_path.to_string_lossy().to_string()
    );

    // point rsync command to it to copy over selected files
    let rsync_status = Command::new("rsync")
        .arg("-avp")
        .arg(&contents_file_arg)
        .arg(path_2_string(from))
        .arg(path_2_string(to))
        .stdout(Stdio::inherit())
        .status()
        .into_diagnostic()?;

    if rsync_status.success() {
        Ok(())
    } else {
        Err(miette::miette!("failed to copy directories with rsync"))
    }
}

fn path_2_string<P: AsRef<Path>>(path: P) -> String {
    path.as_ref().to_string_lossy().to_string()
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
        let flag_value = expand_env(&flag.flag)?;

        if let Some(flag_name) = &flag.flag_name {
            let flag_name = flag_name.to_uppercase();
            if env_flags.contains_key(&flag_name) {
                let flag_ref = env_flags.get_mut(&flag_name).unwrap();
                flag_ref.push_str(" ");
                flag_ref.push_str(&flag_value);
            } else {
                env_flags.insert(flag_name, flag_value.clone());
            }
        } else {
            for flag_name in vec![
                String::from("CFLAGS"),
                String::from("CXXFLAGS"),
                String::from("CPPFLAGS"),
                String::from("FFLAGS"),
            ] {
                if env_flags.contains_key(&flag_name) {
                    let flag_ref = env_flags.get_mut(&flag_name).unwrap();
                    flag_ref.push_str(" ");
                    flag_ref.push_str(&flag_value);
                } else {
                    env_flags.insert(flag_name, flag_value.clone());
                }
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

#[inline(never)]
fn can_expand_value(value: &str) -> bool {
    value.contains("$")
}

#[inline(never)]
fn expand_env(value: &str) -> Result<String> {
    if can_expand_value(value) {
        shellexpand::env(value)
            .map(|r| r.to_string())
            .into_diagnostic()
    } else {
        Ok(value.clone().to_string())
    }
}
