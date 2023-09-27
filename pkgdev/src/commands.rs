pub mod add;
pub mod set;
pub mod workspace;

use bundle::Bundle;
use clap::Parser;
use miette::Result;

use crate::workspace::Workspace;

use self::{add::handle_add, set::handle_set};

#[derive(Debug, Parser)]
pub enum ShellCommands {
    Add {
        #[command(subcommand)]
        section: add::Sections,
    },
    Set {
        #[command(subcommand)]
        section: set::Sections,
    },
    Exit,
}

pub enum CommandReturn {
    Continue,
    Exit,
}

pub fn handle_command(
    args: &ShellCommands,
    wks: &Workspace,
    pkg: &mut Bundle,
) -> Result<CommandReturn> {
    match args {
        ShellCommands::Add { section } => {
            handle_add(wks, &section, pkg)?;
            Ok(CommandReturn::Continue)
        }
        ShellCommands::Exit => Ok(CommandReturn::Exit),
        ShellCommands::Set { section } => {
            handle_set(wks, &section, pkg)?;
            Ok(CommandReturn::Continue)
        }
    }
}
