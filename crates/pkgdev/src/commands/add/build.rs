use clap::Subcommand;

#[derive(Debug, Subcommand, Clone)]
pub enum BuildSection {
    Configure {
        #[arg(long, short)]
        arg: Vec<String>,
    },
    CMake,
    Meson,
    Script {
        arg: Vec<String>,
    },
}

pub(crate) fn handle_section(section: &BuildSection) -> bundle::BuildSection {
    match section {
        BuildSection::Configure { arg } => {
            let options = arg
                .iter()
                .filter_map(|arg| {
                    if arg.starts_with("--") {
                        Some(bundle::BuildOptionNode {
                            option: arg.strip_prefix("--").unwrap().to_owned(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            bundle::BuildSection::Configure(bundle::ConfigureBuildSection {
                options,
                flags: vec![],
                compiler: None,
                linker: None,
            })
        }
        BuildSection::CMake => todo!(),
        BuildSection::Meson => todo!(),
        BuildSection::Script { .. } => todo!(),
    }
}
