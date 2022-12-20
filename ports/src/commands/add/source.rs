use bundle::{ArchiveSource, Bundle, SourceNode};
use clap::Subcommand;
use miette::{IntoDiagnostic, Result};

#[derive(Debug, Subcommand)]
pub enum Sources {
    Archive { url: url::Url },
}

impl std::fmt::Display for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sources::Archive { url } => write!(f, "archive {}", url),
        }
    }
}

pub fn handle_add_source(src: &Sources, pkg: &mut Bundle) -> Result<()> {
    let src_node = match src {
        Sources::Archive { url } => SourceNode::Archive(ArchiveSource {
            src: url.to_string(),
        }),
    };

    pkg.add_source(src_node).into_diagnostic()
}
