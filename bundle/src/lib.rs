use miette::Diagnostic;
use miette::{Context, IntoDiagnostic};
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum BundleError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("no parent directory of package.kdl exists")]
    NoPackageDocumentParentDir,
    #[error(transparent)]
    #[diagnostic(code(bundle::knuffel_error))]
    Knuffel(#[from] knuffel::Error),
}

type BundleResult<T> = std::result::Result<T, BundleError>;

pub struct Bundle {
    path: PathBuf,
    pub package_document: Package,
}

impl Bundle {
    pub fn new<P: AsRef<Path>>(path: P) -> BundleResult<Self> {
        let path = path.as_ref().canonicalize()?;

        let (package_document_string, name) = if path.is_file() {
            (
                read_to_string(path.clone())?,
                path.parent()
                    .ok_or(BundleError::NoPackageDocumentParentDir)?
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            (
                read_to_string(&path.join("package.kdl"))?,
                path.to_string_lossy().to_string(),
            )
        };

        if path.is_file() {
            let package_document = knuffel::parse::<Package>(&name, &package_document_string)?;
            Ok(Self {
                path: path
                    .parent()
                    .ok_or(BundleError::NoPackageDocumentParentDir)?
                    .to_path_buf(),
                package_document,
            })
        } else {
            let package_document = knuffel::parse::<Package>(&name, &package_document_string)?;
            Ok(Self {
                path,
                package_document,
            })
        }
    }
}

#[derive(knuffel::Decode)]
pub struct Package {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(children)]
    pub sections: Vec<Section>,
}

#[derive(knuffel::Decode)]
pub enum Section {
    Sources(SourceSection),
    Build(BuildSection),
}

#[derive(knuffel::Decode)]
pub struct SourceSection {
    #[knuffel(children)]
    pub sources: Vec<SourceNode>,
}

#[derive(knuffel::Decode)]
pub enum SourceNode {
    Archive(ArchiveSource),
    Git(GitSource),
    File(FileSource),
    Patch(PatchSource),
    Overlay(OverlaySource),
}

#[derive(knuffel::Decode)]
pub struct ArchiveSource {
    #[knuffel(argument)]
    pub src: String,
}

#[derive(knuffel::Decode)]
pub struct GitSource {
    #[knuffel(argument)]
    pub repository: String,
    #[knuffel(property)]
    pub branch: Option<String>,
    #[knuffel(property)]
    pub tag: Option<String>,
}

#[derive(knuffel::Decode)]
pub struct FileSource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
    #[knuffel(argument)]
    pub target_path: Option<PathBuf>,
}

#[derive(knuffel::Decode)]
pub struct PatchSource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
    #[knuffel(property)]
    pub drop_directories: Option<i32>,
}

#[derive(knuffel::Decode)]
pub struct OverlaySource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
}

#[derive(knuffel::Decode)]
pub struct BuildSection {}

mod tests {

    use crate::*;

    use std::path::{Path, PathBuf};

    /// Find all the bundle files at the given path. This will search the path
    /// recursively for any file named `package.kdl`.
    #[allow(dead_code)]
    pub fn find_bundle_files(path: &Path) -> BundleResult<Vec<PathBuf>> {
        let mut result = Vec::new();
        find_bundle_files_rec(path, &mut result)?;
        Ok(result)
    }

    /// Search the file system recursively for all build files.
    #[allow(dead_code)]
    fn find_bundle_files_rec(path: &Path, result: &mut Vec<PathBuf>) -> BundleResult<()> {
        for entry in std::fs::read_dir(path)? {
            let e = entry?;
            let ft = e.file_type()?;
            if ft.is_symlink() {
                continue;
            } else if ft.is_dir() {
                find_bundle_files_rec(&e.path(), result)?;
            } else if e.file_name() == "package.kdl" {
                result.push(e.path());
            }
        }

        Ok(())
    }

    #[test]
    fn test_read_all_samples() {
        let paths = find_bundle_files(Path::new("../packages")).unwrap();
        let bundles = paths
            .into_iter()
            .map(|path| match Bundle::new(&path) {
                Ok(b) => b,
                Err(e) => {
                    panic!("could not read bundle package {} {:?}", path.display(), e);
                }
            })
            .collect::<Vec<Bundle>>();
        for bundle in bundles {
            println!("Package: {}", bundle.package_document.name);
        }
    }

    #[test]
    fn parse_one() {
        let bundle_path = Path::new("../packages/openssl/package.kdl");
        let package_document_string = read_to_string(&bundle_path).unwrap();
        let b = match knuffel::parse::<Package>("openssl", &package_document_string) {
            Ok(b) => b,
            Err(e) => {
                assert!(false);
                panic!("{:?}", miette::Report::new(e));
            }
        };
    }
}
