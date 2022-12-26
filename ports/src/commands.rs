pub mod add;
pub mod workspace;

use bundle::Bundle;
use clap::Parser;
use miette::Result;

use crate::workspace::Workspace;

use self::add::handle_add;

#[derive(Debug, Parser)]
pub enum ShellCommands {
    Add {
        #[command(subcommand)]
        section: add::Sections,
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
    doc: &mut Bundle,
) -> Result<CommandReturn> {
    match args {
        ShellCommands::Add { section } => {
            handle_add(wks, &section, doc)?;
            Ok(CommandReturn::Continue)
        }
        ShellCommands::Exit => Ok(CommandReturn::Exit),
    }
}
