mod commands;
mod config;
mod download;
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
        #[arg(long = "cross", short)]
        cross_prefix: Option<PathBuf>,

        /// Select the triple for the Cross Build it must be a supported option
        #[arg(long, short)]
        cross_triple: Option<CrossTriple>,
    },
}

#[derive(Debug, ValueEnum, Clone)]
pub(crate) enum CrossTriple {
    Arm,
    Riscv,
    Sparc,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BuildSteps {
    Download,
    Unpack,
    Patch,
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
            cross_prefix,
            cross_triple,
        } => {
            let wks = conf.get_current_wks()?;

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

            let sources: Vec<SourceSection> = package_bundle
                .package_document
                .sections
                .iter()
                .filter_map(|section| match section {
                    bundle::Section::Source(src) => Some(src.clone()),
                    _ => None,
                })
                .collect();

            download::download_and_verify(&wks, sources.as_slice())?;

            if let Some(stop_on_step) = stop_on_step {
                if stop_on_step == BuildSteps::Download {
                    return Ok(());
                }
            }

            unpack::unpack_sources(
                &wks,
                Some(package_bundle.package_document.name.clone()),
                package_bundle.get_path(),
                sources.as_slice(),
            )?;

            //TODO: patch

            //TODO: configure

            //TODO: build

            //TODO: install

            //TODO: mogrify

            //TODO: publish

            Ok(())
        }
    }
}
