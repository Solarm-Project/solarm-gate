pub mod add;

use bundle::Bundle;
use clap::Parser;
use miette::Result;

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

pub fn handle_command(args: &ShellCommands, doc: &mut Bundle) -> Result<CommandReturn> {
    match args {
        ShellCommands::Add { section } => {
            handle_add(&section, doc)?;
            Ok(CommandReturn::Continue)
        }
        ShellCommands::Exit => Ok(CommandReturn::Exit),
    }
}
