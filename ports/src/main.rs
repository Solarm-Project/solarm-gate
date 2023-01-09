mod commands;
mod compile;
mod config;
mod configure;
mod download;
mod env;
mod install;
mod ips;
mod unpack;
mod workspace;

use bundle::{Bundle, SourceSection};
use clap::{Parser, Subcommand, ValueEnum};
use commands::{handle_command, workspace::handle_workspace, ShellCommands};
use config::Config;
use miette::{IntoDiagnostic, Result, WrapErr};
use rustyline::error::ReadlineError;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long = "package", short = 'p')]
    package: Option<PathBuf>,

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
    Create,
    Edit {
        unmatched: Option<Vec<String>>,
    },
    Workspace {
        #[command(subcommand)]
        cmd: Option<commands::workspace::Command>,
    },
    Build {
        #[arg(long = "step", short)]
        stop_on_step: Option<BuildSteps>,

        /// Perform a build that creates cross build compatible tools and install them into the following prefix on this host
        #[arg(long)]
        prefix: Option<PathBuf>,

        /// Select the triple for the Cross Build it must be a supported option
        #[arg(long)]
        cross_triple: Option<CrossTriple>,

        #[arg(long, default_value = "false")]
        clean: bool,
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
    Configure,
    Build,
    Install,
    Mogrify,
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
        Command::Create => {
            let path = if let Some(package) = cli.package {
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
            println!("created package: {}", path.display());
            Ok(())
        }
        Command::Edit { unmatched } => {
            let path = if let Some(package) = cli.package {
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

            let mut package_bundle = Bundle::new(path)?;

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
            prefix: cross_prefix,
            cross_triple,
            clean,
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

            let path = if let Some(package) = cli.package {
                package
            } else {
                Path::new("./").to_path_buf()
            };
            let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                "Can not canonicalize path to package {}",
                path.display()
            ))?;

            let package_bundle = Bundle::new(path)?;

            let sources: Vec<SourceSection> = package_bundle.package_document.sources.clone();

            download::download_and_verify(&wks, sources.as_slice())
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

            configure::configure_package_sources(&wks, &package_bundle, cross_prefix, cross_triple)
                .wrap_err("configure step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Configure {
                    return Ok(());
                }
            }

            compile::run_compile(&wks, &package_bundle).wrap_err("compilation step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Build {
                    return Ok(());
                }
            }

            install::run_install(&wks, &package_bundle).wrap_err("installation step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Install {
                    return Ok(());
                }
            }

            ips::run_generate_filelist(&wks, &package_bundle)
                .wrap_err("generating filelist failed")?;
            ips::run_mogrify(&wks, &package_bundle).wrap_err("mogrify failed")?;
            ips::run_generate_pkgdepend(&wks, &package_bundle)
                .wrap_err("failed to generate dependency entries")?;
            ips::run_resolve_dependencies(&wks, &package_bundle)
                .wrap_err("failed to resolve dependencies")?;
            ips::run_lint(&wks, &package_bundle).wrap_err("lint failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Mogrify {
                    return Ok(());
                }
            }

            //TODO: publish

            Ok(())
        }
    }
}
