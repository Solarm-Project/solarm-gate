use clap::Subcommand;
use miette::Result;

#[derive(Debug, Subcommand)]
pub enum Sources {
    Archive { url: url::Url },
}

impl std::fmt::Display for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceSectionType::Archive { url } => write!(f, "archive {}", url),
        }
    }
}

pub fn handle_add_source(src: Sections::Source) -> Result<()> {
    println!("{}", source_type);

    Ok(())
}
