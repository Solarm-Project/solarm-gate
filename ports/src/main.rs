use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result, WrapErr};
use rustyline::error::ReadlineError;
use std::{fs::create_dir_all, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Create { path: PathBuf },
    Edit { path: PathBuf },
}

#[derive(Debug, Error)]
enum PortsError {
    #[error("can not get basename of the package: does it exist?")]
    CannotGetBaseNameOfPackage,
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Command::Create { path } => {
            if !path.exists() {
                create_dir_all(&path).into_diagnostic().wrap_err(format!(
                    "could not create package directory {}",
                    path.display()
                ))?;
            }
            println!("created package: {}", path.display());
        }
        Command::Edit { path } => {
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
            let ps1 = format!("{}$ ", basename);
            let mut rl = rustyline::Editor::<()>::new().into_diagnostic()?;
            loop {
                let readline = rl.readline(&ps1);
                match readline {
                    Ok(line) => {
                        rl.add_history_entry(line.as_str());
                        println!("Line: {}", line);
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
        }
    }

    Ok(())
}
