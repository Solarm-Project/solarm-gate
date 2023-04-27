mod build;
mod commands;
mod compile;
mod config;
mod download;
mod forge;
mod install;
mod ips;
mod path;
mod tarball;
mod unpack;
mod workspace;

use crate::config::Settings;
use bundle::{Bundle, SourceSection};
use clap::{Parser, Subcommand, ValueEnum};
use commands::{handle_command, workspace::handle_workspace, ShellCommands};
use gate::Gate;
use miette::{IntoDiagnostic, Result, WrapErr};
use rustyline::error::ReadlineError;
use std::{
    fs::create_dir_all,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use workspace::Workspace;

#[derive(Debug, Parser)]
struct Cli {
    /// Allows one to change the workspace for this operation only. Intended for the CI usecase so that
    /// multiple jobs can be run simultaneously
    #[arg(long, short)]
    workspace: Option<String>,

    #[command(subcommand)]
    command: Command,
}

//TODO: Fix command to fix commonly occuring lint errors so it get's easier to package stuff
//TODO: verify command to verify the bundle for common problems.

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Add { name: String, value: String },
    Remove { name: String, value: String },
    Set { name: String, value: String },
    Get { name: String },
}

#[derive(Debug, Subcommand)]
enum Command {
    Create {
        #[arg(long, short)]
        package: Option<PathBuf>,
        name: String,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Edit {
        #[arg(long, short)]
        package: Option<PathBuf>,

        unmatched: Option<Vec<String>>,
    },
    Workspace {
        #[command(subcommand)]
        cmd: Option<commands::workspace::Command>,
    },
    Build {
        #[arg(long = "step", short)]
        stop_on_step: Option<BuildSteps>,

        #[arg(long, short)]
        gate: Option<PathBuf>,

        package: Option<String>,

        #[arg(long, default_value = "false")]
        no_clean: bool,

        #[arg(long, default_value = "false")]
        archive_clean: bool,

        #[arg(short = 'I', long = "include")]
        transform_include_dir: Option<PathBuf>,
    },
    Forge {
        #[command(subcommand)]
        cmd: forge::ForgeCLI,
    },
    /// Show the repository information
    Info {
        /// If set will save the information in the directory of the package.kdl file as json
        /// mainly used to generate repology data
        #[arg(long)]
        save: bool,

        #[arg(long, short)]
        gate: Option<PathBuf>,

        package: Option<String>,
    },
}

#[derive(Debug, ValueEnum, Clone)]
pub enum CrossTriple {
    Arm,
    Riscv,
    Sparc,
}

impl ToString for CrossTriple {
    fn to_string(&self) -> String {
        match self {
            CrossTriple::Arm => String::from("arm"),
            CrossTriple::Riscv => String::from("riscv"),
            CrossTriple::Sparc => String::from("sparc"),
        }
    }
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BuildSteps {
    Download,
    Unpack,
    Build,
    Pack,
    Publish,
}

#[derive(Debug, Error)]
enum PortsError {
    #[error("can not get basename of the package: does it exist?")]
    CannotGetBaseNameOfPackage,
}

pub fn derive_source_name(package_name: String, src: &SourceSection) -> String {
    if let Some(name) = &src.name {
        name.clone().replace("/", "_").to_string()
    } else {
        package_name.replace("/", "_")
    }
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let settings = Settings::open()?;

    match cli.command {
        Command::Workspace { cmd } => match cmd {
            Some(cmd) => match handle_workspace(&cmd)? {
                commands::workspace::WorkspaceReturn::Change { wks } => {
                    println!("changed workspace to: {}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::Current(wks) => {
                    println!("{}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::List(list) => {
                    for wks in list {
                        println!("{}", wks);
                    }
                    Ok(())
                }
            },
            None => match handle_workspace(&commands::workspace::Command::Current)? {
                commands::workspace::WorkspaceReturn::Change { .. } => todo!(),
                commands::workspace::WorkspaceReturn::Current(wks) => {
                    println!("{}", wks.get_name());
                    Ok(())
                }
                commands::workspace::WorkspaceReturn::List(_) => todo!(),
            },
        },
        Command::Create { package, name } => {
            let path = if let Some(package) = package {
                package
            } else {
                Path::new("./").to_path_buf()
            };

            if !path.exists() {
                create_dir_all(&path).into_diagnostic().wrap_err(format!(
                    "could not create package directory {}",
                    path.display()
                ))?;
            }

            let pkg = bundle::PackageBuilder::default()
                .name(name.clone())
                .project_name(name)
                .build()
                .into_diagnostic()
                .wrap_err(miette::miette!("could not create new bundle"))?;

            let doc = pkg.to_document();
            let mut pkg_file = std::fs::File::create(path.join("package.kdl")).into_diagnostic()?;
            pkg_file
                .write_all(doc.to_string().as_bytes())
                .into_diagnostic()
                .wrap_err(miette::miette!("could not save package.kdl"))?;

            println!("created package: {}", path.display());
            Ok(())
        }
        Command::Edit { unmatched, package } => {
            let path = if let Some(package) = package {
                package
            } else {
                Path::new("./").to_path_buf()
            };
            let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                "Can not canonicalize path to package {}",
                path.display()
            ))?;
            let basename = path
                .file_name()
                .ok_or(PortsError::CannotGetBaseNameOfPackage)
                .into_diagnostic()?
                .to_string_lossy()
                .to_string();

            let mut package_bundle = Bundle::open_local(path)?;

            if let Some(unmatched) = unmatched {
                let mut args = vec!["ports"];
                let mut argn = unmatched.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                args.append(&mut argn);
                let wks = settings.get_current_wks()?;
                let cmd: ShellCommands = ShellCommands::parse_from(args);
                match handle_command(&cmd, &wks, &mut package_bundle) {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        return Err(err);
                    }
                }
            }

            let ps1 = format!("{}$ ", basename);
            let mut rl = rustyline::Editor::<()>::new().into_diagnostic()?;
            let wks = Box::new(settings.get_current_wks()?);
            loop {
                let readline = rl.readline(&ps1);
                match readline {
                    Ok(line) => {
                        let mut args = vec!["shell"];
                        args.append(&mut line.split(" ").collect());
                        let cmd: ShellCommands = match ShellCommands::try_parse_from(args) {
                            Ok(cmd) => cmd,
                            Err(e) => {
                                eprintln!("{}", e);
                                continue;
                            }
                        };

                        match handle_command(&cmd, &wks, &mut package_bundle) {
                            Ok(res) => match res {
                                commands::CommandReturn::Continue => {}
                                commands::CommandReturn::Exit => break,
                            },
                            Err(err) => {
                                eprintln!("{}", err);
                            }
                        }
                    }
                    Err(ReadlineError::Interrupted) => {
                        println!("CTRL-C");
                        break;
                    }
                    Err(ReadlineError::Eof) => {
                        println!("CTRL-D");
                        break;
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
            Ok(())
        }
        Command::Build {
            stop_on_step,
            no_clean,
            archive_clean,
            gate,
            package,
            transform_include_dir,
        } => {
            let wks = if let Some(wks_path) = cli.workspace {
                settings.get_workspace_from(&wks_path)?
            } else {
                settings.get_current_wks()?
            };

            let transform_include_dir = transform_include_dir.map(|p| match p.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    println!(
                        "could not cannonicalize {} due to {} continuing ignoring and continuing",
                        p.display(),
                        e
                    );
                    p
                }
            });

            if !no_clean {
                std::fs::remove_dir_all(wks.get_or_create_download_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the download directory")?;
                std::fs::remove_dir_all(wks.get_or_create_build_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the build directory")?;
                std::fs::remove_dir_all(wks.get_or_create_prototype_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the prototype directory")?;
                std::fs::remove_dir_all(wks.get_or_create_manifest_dir()?)
                    .into_diagnostic()
                    .wrap_err("could not clean the manifest directory")?;
            }

            let (package_bundle, gate_data) = if let Some(gate_path) = gate {
                let gate_data = gate::Gate::new(&gate_path).wrap_err("could not open gate data")?;

                let path = if let Some(package) = &package {
                    let name = if package.contains("/") {
                        package.rsplit_once('/').unwrap().1
                    } else {
                        package.as_str()
                    };
                    gate_path
                        .parent()
                        .unwrap_or(Path::new("./"))
                        .join("packages")
                        .join(name)
                } else {
                    Path::new("./").to_path_buf()
                };

                let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                    "Can not canonicalize path to package {}",
                    path.display()
                ))?;

                let mut package_bundle =
                    Bundle::open_local(path).wrap_err("could not open package.kdl of package")?;

                if let Some(package) = &package {
                    if let Some(gate_package) = gate_data.get_package(package.as_str()) {
                        package_bundle
                            .package_document
                            .merge_into_mut(&gate_package)?;
                    }
                }

                (package_bundle, Some(gate_data))
            } else {
                let path = if let Some(package) = package {
                    let name = if package.contains("/") {
                        package.split_once('/').unwrap().1
                    } else {
                        package.as_str()
                    };
                    Path::new("./packages").join(name)
                } else {
                    Path::new("./").to_path_buf()
                };

                let path = path.canonicalize().into_diagnostic().wrap_err(format!(
                    "Can not canonicalize path to package {}",
                    path.display()
                ))?;

                (
                    Bundle::open_local(path).wrap_err("could not open package.kdl of package")?,
                    None,
                )
            };

            let sources: Vec<SourceSection> = package_bundle.package_document.sources.clone();

            download::download_and_verify(&wks, sources.as_slice(), archive_clean)
                .wrap_err("download and verify failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Download {
                    return Ok(());
                }
            }

            unpack::unpack_sources(
                &wks,
                package_bundle.package_document.name.clone(),
                package_bundle.get_path(),
                sources.as_slice(),
            )
            .wrap_err("unpack step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Unpack {
                    return Ok(());
                }
            }

            build::build_package_sources(&wks, &package_bundle, &settings)
                .wrap_err("configure step failed")?;

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Build {
                    return Ok(());
                }
            }

            if let Some(gate_data) = gate_data {
                if let Some(distribution) = &gate_data.distribution {
                    match distribution.distribution_type {
                        gate::DistributionType::Tarbball => {
                            tarball::make_release_tarball(&wks, &package_bundle)?;
                        }
                        gate::DistributionType::IPS => {
                            run_ips_actions(
                                &wks,
                                &package_bundle,
                                Some(gate_data),
                                transform_include_dir,
                            )?;
                        }
                    }
                } else {
                    run_ips_actions(
                        &wks,
                        &package_bundle,
                        Some(gate_data),
                        transform_include_dir,
                    )?;
                }
            } else {
                run_ips_actions(
                    &wks,
                    &package_bundle,
                    gate_data.clone(),
                    transform_include_dir,
                )?;
            }

            if let Some(stop_on_step) = &stop_on_step {
                if stop_on_step == &BuildSteps::Pack {
                    return Ok(());
                }
            }

            Ok(())
        }
        Command::Forge { cmd } => forge::handle_forge(&cmd),
        Command::Config { command } => {
            let mut cfg = Settings::open()?;
            match command {
                ConfigCommand::Set { name, .. } => {
                    match name.as_str() {
                        "test" => {
                            println!("just testing code, nothing will happen here");
                        }
                        x => {
                            return Err(miette::miette!(format!(
                                "no setting named {} is changeable like this",
                                x
                            )));
                        }
                    };
                }
                ConfigCommand::Get { name } => match name.as_str() {
                    "output" => println!("output_dir={}", cfg.get_output_dir()),
                    _ => {
                        return Err(miette::miette!("unknown value cannot get from config"));
                    }
                },
                ConfigCommand::Add { name, value } => match name.as_str() {
                    "search_path" => {
                        cfg.add_path_to_search(value);
                    }
                    _ => {
                        return Err(miette::miette!("cannot set this config from here"));
                    }
                },
                ConfigCommand::Remove { name, value } => match name.as_str() {
                    "search_path" => {
                        cfg.remove_path_from_search(value);
                    }
                    _ => {
                        return Err(miette::miette!("cannot set this config from here"));
                    }
                },
            };

            cfg.save()?;
            Ok(())
        }
        Command::Info {
            save,
            package,
            gate,
        } => {
            let path = if let Some(package) = package {
                Path::new(package.as_str()).to_path_buf()
            } else {
                Path::new("./").to_path_buf()
            };

            let gate = if let Some(gate_file) = &gate {
                Gate::new(gate_file)?
            } else {
                Gate::default()
            };

            let pkg = Bundle::open_local(path)?;

            let mut builder = repology::MetadataBuilder::default();

            if let Some(summary) = &pkg.package_document.summary {
                builder.summary(summary);
            }

            builder.maintainers(pkg.package_document.maintainers.clone());

            builder.project_name(&pkg.package_document.project_name.clone());

            builder.source_name(&pkg.get_name());

            if let Some(hp) = &pkg.package_document.project_url {
                builder.add_homepage(hp);
            }

            if let Some(license) = &pkg.package_document.license {
                builder.add_license(license);
            }

            builder.source_links(
                pkg.package_document
                    .sources
                    .iter()
                    .map(|s| -> Vec<String> {
                        s.sources
                            .iter()
                            .filter_map(|s| -> Option<String> {
                                match s {
                                    bundle::SourceNode::Archive(s) => Some(s.src.clone()),
                                    bundle::SourceNode::Git(g) => Some(g.repository.clone()),
                                    bundle::SourceNode::File(_) => None,
                                    bundle::SourceNode::Directory(_) => None,
                                    bundle::SourceNode::Patch(_) => None,
                                    bundle::SourceNode::Overlay(_) => None,
                                }
                            })
                            .collect()
                    })
                    .flatten()
                    .collect::<Vec<String>>()
                    .clone(),
            );

            if let Some(cat) = &pkg.package_document.classification {
                builder.add_category(cat);
            }

            if let Some(vers) = &pkg.package_document.version {
                builder.version(semver::Version::parse(vers).into_diagnostic()?);
            }

            builder.fmri(format!(
                "pkg:/{name}@{version},{build_version}-{branch_version}.{revision}",
                name = &pkg.package_document.name,
                version = &pkg
                    .package_document
                    .version
                    .ok_or(miette::miette!("no version field spcified in package.kdl"))?,
                build_version = gate.version,
                branch_version = gate.branch,
                revision = &pkg.package_document.revision.unwrap_or(String::from("0")),
            ));

            let data = builder.build().into_diagnostic()?;

            let data_string = serde_json::to_string_pretty(&data).into_diagnostic()?;

            if save {
                todo!();
            }

            println!("{}\n", &data_string);

            Ok(())
        }
    }
}

fn run_ips_actions(
    wks: &Workspace,
    pkg: &Bundle,
    gate_data: Option<Gate>,
    transform_include_dir: Option<PathBuf>,
) -> miette::Result<()> {
    ips::run_generate_filelist(wks, pkg).wrap_err("generating filelist failed")?;
    ips::run_mogrify(wks, pkg, gate_data.clone(), transform_include_dir)
        .wrap_err("mogrify failed")?;
    ips::run_generate_pkgdepend(wks, pkg).wrap_err("failed to generate dependency entries")?;
    ips::run_resolve_dependencies(wks, pkg).wrap_err("failed to resolve dependencies")?;
    ips::run_lint(wks, pkg).wrap_err("lint failed")?;

    let publisher = &gate_data.unwrap_or(Gate::default()).publisher;
    ips::ensure_repo_with_publisher_exists(&publisher)
        .wrap_err("failed to ensure repository exists")?;
    ips::publish_package(wks, pkg, &publisher).wrap_err("package publish failed")?;
    Ok(())
}
