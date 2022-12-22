mod commands;
mod workspace;

use bundle::Bundle;
use clap::{Parser, Subcommand};
use commands::{handle_command, ShellCommands};
use miette::{IntoDiagnostic, Result, WrapErr};
use rustyline::error::ReadlineError;
use std::{fs::create_dir_all, path::PathBuf};
use thiserror::Error;
use workspace::Workspace;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long = "package", short = 'p')]
    package: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Create,
    Edit { unmatched: Option<Vec<String>> },
}

#[derive(Debug, Error)]
enum PortsError {
    #[error("can not get basename of the package: does it exist?")]
    CannotGetBaseNameOfPackage,
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Command::Create => {
            let path = cli.package;
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
            let path = cli.package;
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
                let wks = Workspace::new("./")?;
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
            loop {
                let readline = rl.readline(&ps1);
                match readline {
                    Ok(line) => {
                        let mut args = vec!["shell"];
                        args.append(&mut line.split(" ").collect());
                        let wks = Workspace::new("./")?;
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
    }
}
