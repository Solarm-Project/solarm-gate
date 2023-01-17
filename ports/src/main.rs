mod build;
mod commands;
mod compile;
mod config;
mod download;
mod env;
mod install;
mod ips;
mod tarball;
mod unpack;
mod workspace;

use bundle::{Bundle, SourceSection};
use clap::{Parser, Subcommand, ValueEnum};
use commands::{handle_command, workspace::handle_workspace, ShellCommands};
use config::Config;
use gate::Gate;
use miette::{IntoDiagnostic, Result, WrapErr};
use rustyline::error::ReadlineError;
use std::{
    fs::create_dir_all,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use workspace::Workspace;

#[derive(Debug, Parser)]
struct Cli {
    /// Allows one to change the workspace for this operation only. Intended for the CI usecase so that
    /// multiple jobs can be run simultaneously
    #[arg(long, short)]
    workspace: Option<String>,

    #[command(subcommand)]
    command: Command,
}

//TODO: Fix command to fix commonly occuring lint errors so it get's easier to package stuff
//TODO: verify command to verify the bundle for common problems.

#[derive(Debug, Subcommand)]
enum Command {
    Create {
        #[arg(long, short)]
        package: Option<PathBuf>,
        name: String,
    },
    Edit {
        #[arg(long, short)]
        package: Option<PathBuf>,

        unmatched: Option<Vec<String>>,
    },
    Workspace {
        #[command(subcommand)]
        cmd: Option<commands::workspace::Command>,
    },
    Build {
        #[arg(long = "step", short)]
        stop_on_step: Option<BuildSteps>,

        #[arg(long, short)]
        gate: Option<PathBuf>,

        #[arg(long, short)]
        package: Option<String>,

        #[arg(long, default_value = "false")]
        clean: bool,

        #[arg(long, default_value = "false")]
        archive_clean: bool,
    },
}

#[derive(Debug, ValueEnum, Clone)]
pub enum CrossTriple {
    Arm,
    Riscv,
    Sparc,
}

impl ToString for CrossTriple {
    fn to_string(&self) -> String {
        match self {
            CrossTriple::Arm => String::from("arm"),
            CrossTriple::Riscv => String::from("riscv"),
            CrossTriple::Sparc => String::from("sparc"),
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BuildSteps {
    Download,
    Unpack,
    Build,
    GenerateManifests,
    Publish,
}

#[derive(Debug, Error)]
enum PortsError {
    #[error("can not get basename of the package: does it exist?")]
    CannotGetBaseNameOfPackage,
}

pub fn derive_source_name(package_name: String, src: &SourceSection) -> String {
    if let Some(name) = &src.name {
        name.clone().replace("/", "_").to_string()
    } else {
        package_name.replace("/", "_")
    }
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let conf = Config::open()?;

    match cli.command {
        Command::Workspace { cmd } => match cmd {
            Some(cmd) => match handle_workspace(&cmd)? {
                commands::workspace::WorkspaceReturn::Change { wks } => {
                    println!("changed workspace to: {}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::Current(wks) => {
                    println!("{}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::List(list) => {
                    for wks in list {
                        println!("{}", wks);
                    }
                    Ok(())
                }
            },
            None => match handle_workspace(&commands::workspace::Command::Current)? {
                commands::workspace::WorkspaceReturn::Change { .. } => todo!(),
                commands::workspace::WorkspaceReturn::Current(wks) => {
                    println!("{}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::List(_) => todo!(),
            },
        },
        Command::Create { package, name } => {
            let path = if let Some(package) = package {
                package
            } else {
                Path::new("./").to_path_buf()
            };

            if !path.exists() {
                create_dir_all(&path).into_diagnostic().wrap_err(format!(
                    "could not create package directory {}",
                    path.display()
                ))?;
            }

            let mut doc = kdl::KdlDocument::new();
            let mut name_node = kdl::KdlNode::new("name");
            name_node.insert(0, name.as_str());
            doc.nodes_mut().push(name_node);
            let mut pkg_file = std::fs::File::create(path.join("package.kdl")).into_diagnostic()?;
            pkg_file
                .write_all(doc.to_string().as_bytes())
                .into_diagnostic()?;

            println!("created package: {}", path.display());
            Ok(())
        }
        Command::Edit { unmatched, package } => {
            let path = if let Some(package) = package {
                package
            } else {
                Path::new("./").to_path_buf()
            };
            let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                "Can not canonicalize path to package {}",
                path.display()
            ))?;
            let basename = path
                .file_name()
                .ok_or(PortsError::CannotGetBaseNameOfPackage)
                .into_diagnostic()?
                .to_string_lossy()
                .to_string();

            let mut package_bundle = Bundle::open_local(path)?;

            if let Some(unmatched) = unmatched {
                let mut args = vec!["ports"];
                let mut argn = unmatched.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                args.append(&mut argn);
                let wks = conf.get_current_wks()?;
                let cmd: ShellCommands = ShellCommands::parse_from(args);
                match handle_command(&cmd, &wks, &mut package_bundle) {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        return Err(err);
                    }
                }
            }

            let ps1 = format!("{}$ ", basename);
            let mut rl = rustyline::Editor::<()>::new().into_diagnostic()?;
            let wks = Box::new(conf.get_current_wks()?);
            loop {
                let readline = rl.readline(&ps1);
                match readline {
                    Ok(line) => {
                        let mut args = vec!["shell"];
                        args.append(&mut line.split(" ").collect());
                        let cmd: ShellCommands = match ShellCommands::try_parse_from(args) {
                            Ok(cmd) => cmd,
                            Err(e) => {
                                eprintln!("{}", e);
                                continue;
                            }
                        };

                        match handle_command(&cmd, &wks, &mut package_bundle) {
                            Ok(res) => match res {
                                commands::CommandReturn::Continue => {}
                                commands::CommandReturn::Exit => break,
                            },
                            Err(err) => {
                                eprintln!("{}", err);
                            }
                        }
                    }
                    Err(ReadlineError::Interrupted) => {
                        println!("CTRL-C");
                        break;
                    }
                    Err(ReadlineError::Eof) => {
                        println!("CTRL-D");
                        break;
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
            Ok(())
        }
        Command::Build {
            stop_on_step,
            clean,
            archive_clean,
            gate,
            package,
        } => {
            let wks = if let Some(wks_path) = cli.workspace {
                conf.get_workspace_from(&wks_path)?
            } else {
                conf.get_current_wks()?
            };

            if clean {
                std::fs::remove_dir_all(wks.get_or_create_download_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the download directory")?;
                std::fs::remove_dir_all(wks.get_or_create_build_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the build directory")?;
                std::fs::remove_dir_all(wks.get_or_create_prototype_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the prototype directory")?;
                std::fs::remove_dir_all(wks.get_or_create_manifest_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the manifest directory")?;
            }

            let (package_bundle, gate_data) = if let Some(gate_path) = gate {
                let gate_data = gate::Gate::new(&gate_path)?;

                let path = if let Some(package) = &package {
                    let name = if package.contains("/") {
                        package.split_once('/').unwrap().1
                    } else {
                        package.as_str()
                    };
                    gate_path
                        .parent()
                        .unwrap_or(Path::new("./"))
                        .join("packages")
                        .join(name)
                } else {
                    Path::new("./").to_path_buf()
                };

                let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                    "Can not canonicalize path to package {}",
                    path.display()
                ))?;

                let mut package_bundle = Bundle::open_local(path)?;

                if let Some(package) = &package {
                    if let Some(gate_package) = gate_data.get_package(package.as_str()) {
                        package_bundle
                            .package_document
                            .merge_into_mut(&gate_package);
                    }
                }

                (package_bundle, Some(gate_data))
            } else {
                let path = if let Some(package) = package {
                    let name = if package.contains("/") {
                        package.split_once('/').unwrap().1
                    } else {
                        package.as_str()
                    };
                    Path::new("./packages").join(name)
                } else {
                    Path::new("./").to_path_buf()
                };

                let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                    "Can not canonicalize path to package {}",
                    path.display()
                ))?;

                (Bundle::open_local(path)?, None)
            };

            let sources: Vec<SourceSection> = package_bundle.package_document.sources.clone();

            download::download_and_verify(&wks, sources.as_slice(), archive_clean)
                .wrap_err("download and verify failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Download {
                    return Ok(());
                }
            }

            unpack::unpack_sources(
                &wks,
                package_bundle.package_document.name.clone(),
                package_bundle.get_path(),
                sources.as_slice(),
            )
            .wrap_err("unpack step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Unpack {
                    return Ok(());
                }
            }

            build::build_package_sources(&wks, &package_bundle)
                .wrap_err("configure step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Build {
                    return Ok(());
                }
            }

            if let Some(gate_data) = gate_data {
                if let Some(distribution) = &gate_data.distribution {
                    match distribution.distribution_type {
                        gate::DistributionType::Tarbball => {
                            tarball::make_release_tarball(&wks, &package_bundle)?;
                        }
                        gate::DistributionType::IPS => {
                            run_ips_actions(&wks, &package_bundle, Some(gate_data))?;
                        }
                    }
                } else {
                    run_ips_actions(&wks, &package_bundle, Some(gate_data))?;
                }
            } else {
                run_ips_actions(&wks, &package_bundle, gate_data.clone())?;
            }

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::GenerateManifests {
                    return Ok(());
                }
            }

            //TODO: publish

            Ok(())
        }
    }
}

fn run_ips_actions(wks: &Workspace, pkg: &Bundle, gate_data: Option<Gate>) -> miette::Result<()> {
    ips::run_generate_filelist(wks, pkg).wrap_err("generating filelist failed")?;
    ips::run_mogrify(wks, pkg, gate_data).wrap_err("mogrify failed")?;
    ips::run_generate_pkgdepend(wks, pkg).wrap_err("failed to generate dependency entries")?;
    ips::run_resolve_dependencies(wks, pkg).wrap_err("failed to resolve dependencies")?;
    ips::run_lint(wks, pkg).wrap_err("lint failed")?;

    Ok(())
}
