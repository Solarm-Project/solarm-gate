mod build;
mod source;

use bundle::Bundle;
use clap::Subcommand;
use miette::Result;

use crate::workspace::Workspace;

#[derive(Debug, Subcommand)]
pub enum Sections {
    Source {
        #[command(subcommand)]
        source: source::Sources,
    },
    Maintainer {
        name: String,
    },
    Dependency {
        name: String,

        #[arg(long, short)]
        kind: Option<DependencyKind>,

        #[arg(long, short)]
        dev: bool,
    },
    Build {
        #[command(subcommand)]
        section: build::BuildSection,
    },
}

#[derive(Debug, clap::ValueEnum, Clone)]
pub enum DependencyKind {
    Require,
    Incorporate,
    Optional,
}

impl From<DependencyKind> for bundle::DependencyKind {
    fn from(value: DependencyKind) -> Self {
        match value {
            DependencyKind::Require => bundle::DependencyKind::Require,
            DependencyKind::Incorporate => bundle::DependencyKind::Incorporate,
            DependencyKind::Optional => bundle::DependencyKind::Optional,
        }
    }
}

pub fn handle_add(wks: &Workspace, section: &Sections, doc: &mut Bundle) -> Result<()> {
    match section {
        Sections::Source { source } => source::handle_add_source(wks, &source, doc),
        Sections::Maintainer { name } => {
            doc.package_document.maintainers.push(name.clone());
            Ok(())
        }
        Sections::Dependency { name, kind, dev } => {
            doc.package_document.dependencies.push(bundle::Dependency {
                name: name.clone(),
                dev: dev.clone(),
                kind: kind.clone().map(|k| k.into()),
            });
            Ok(())
        }
        Sections::Build { section } => {
            let section = build::handle_section(section);
            doc.package_document.add_build_section(section);
            Ok(())
        }
    }
}
