mod source;

use bundle::Bundle;
use clap::Subcommand;
use miette::Result;

#[derive(Debug, Subcommand)]
pub enum Sections {
    Source {
        #[command(subcommand)]
        source: source::Sources,
    },
}

impl std::fmt::Display for Sections {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sections::Source { source } => write!(f, "source {}", source),
        }
    }
}

pub fn handle_add(section: &Sections, doc: &mut Bundle) -> Result<()> {
    match section {
        Sections::Source { source } => source::handle_add_source(&source, doc),
    }
}
