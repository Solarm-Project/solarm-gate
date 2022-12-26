use clap::Subcommand;
use miette::Result;

use crate::config::Config;

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Current,
    Change { name: String },
    List,
}

pub enum WorkspaceReturn {
    Change { wks: crate::workspace::Workspace },
    Current(crate::workspace::Workspace),
    List(Vec<String>),
}

pub fn handle_workspace(cmd: &Command) -> Result<WorkspaceReturn> {
    match cmd {
        Command::Current => {
            let conf = Config::open()?;
            Ok(WorkspaceReturn::Current(conf.get_current_wks()?))
        }
        Command::Change { name } => {
            let mut conf = Config::open()?;
            let wks = conf.change_current_workspace(&name)?;
            Ok(WorkspaceReturn::Change { wks })
        }
        Command::List => {
            let workspaces = Config::list_workspaces()?;
            Ok(WorkspaceReturn::List(workspaces))
        }
    }
}
