use clap::Subcommand;

#[derive(Debug, Subcommand, Clone)]
pub enum BuildSection {
    Configure {
        #[arg(long, short)]
        options: String,

        #[arg(long, short)]
        flags: String,
    },
    CMake,
    Meson,
    Script {
        scripts: Vec<String>,
    },
}

pub(crate) fn handle_section(section: &BuildSection) -> bundle::BuildSection {
    match section {
        BuildSection::Configure { options, flags } => {
            let options = options
                .split("--")
                .map(|opt| {
                    if let Some((key, value)) = opt.split_once(' ') {
                        bundle::BuildOptionNode {
                            option: format!("{}={}", key, value),
                        }
                    } else {
                        bundle::BuildOptionNode {
                            option: opt.to_owned(),
                        }
                    }
                })
                .collect::<Vec<bundle::BuildOptionNode>>();
            let flags = flags
                .split(' ')
                .map(|flag| {
                    if let Some((name, values)) = flag.split_once('=') {
                        bundle::BuildFlagNode {
                            flag: values.to_owned(),
                            flag_name: Some(name.to_owned()),
                        }
                    } else {
                        bundle::BuildFlagNode {
                            flag: flag.to_owned(),
                            flag_name: None,
                        }
                    }
                })
                .collect::<Vec<bundle::BuildFlagNode>>();
            bundle::BuildSection::Configure(bundle::ConfigureBuildSection {
                options,
                flags,
                compiler: None,
                linker: None,
            })
        }
        BuildSection::CMake => todo!(),
        BuildSection::Meson => todo!(),
        BuildSection::Script { .. } => todo!(),
    }
}
