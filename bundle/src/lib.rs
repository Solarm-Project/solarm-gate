use kdl::KdlDocument;
use miette::Diagnostic;
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug, Diagnostic)]
pub enum BundleError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("no parent directory of package.kdl exists")]
    NoPackageDocumentParentDir,
    #[error(transparent)]
    #[diagnostic(code(bundle::knuffel_error))]
    Knuffel(#[from] knuffel::Error),
    #[error(transparent)]
    #[diagnostic(code(bundle::kdl_error))]
    Kdl(#[from] kdl::KdlError),
    #[error(transparent)]
    #[diagnostic(code(bundle::url_parse_error))]
    UrlParseError(#[from] url::ParseError),
}

type BundleResult<T> = std::result::Result<T, BundleError>;

pub struct Bundle {
    path: PathBuf,
    pub package_document: Package,
    kdl_doc: Option<KdlDocument>,
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
                kdl_doc: None,
            })
        } else {
            let package_document = knuffel::parse::<Package>(&name, &package_document_string)?;
            Ok(Self {
                path,
                package_document,
                kdl_doc: None,
            })
        }
    }

    fn open_document(&mut self) -> BundleResult<()> {
        let data_string = read_to_string(&self.path.join("package.kdl"))?;
        self.kdl_doc = Some(data_string.parse()?);
        Ok(())
    }

    fn save_document(&mut self) -> BundleResult<()> {
        let doc_str = self.kdl_doc.as_ref().unwrap().to_string();
        let mut f = File::create(&self.path.join("package.kdl"))?;
        f.write_all(doc_str.as_bytes())?;
        Ok(())
    }

    pub fn add_source(&mut self, node: SourceNode) -> BundleResult<()> {
        let kdl_doc = if let Some(kdl_doc) = &mut self.kdl_doc {
            kdl_doc
        } else {
            self.open_document()?;
            self.kdl_doc.as_mut().unwrap()
        };

        if kdl_doc.get("sources").is_none() {
            kdl_doc.nodes_mut().push(kdl::KdlNode::new("sources"))
        }

        let src_node: &mut kdl::KdlNode = kdl_doc.get_mut("sources").unwrap();
        let src_nodes = src_node.ensure_children();

        match node {
            SourceNode::Archive(src) => {
                let archive_source: Url = src.src.parse()?;
                let mut n = kdl::KdlNode::new("archive");
                n.push(kdl::KdlEntry::new(archive_source.to_string()));
                n.push(kdl::KdlEntry::new_prop("sha512", src.sha512));
                src_nodes.nodes_mut().push(n);
                self.save_document()?;
            }
            SourceNode::Git(git_src) => {
                let mut n = kdl::KdlNode::new("git");
                n.push(kdl::KdlEntry::new(git_src.repository));
                if let Some(branch) = git_src.branch {
                    n.push(kdl::KdlEntry::new_prop("branch", branch));
                }
                if let Some(tag) = git_src.tag {
                    n.push(kdl::KdlEntry::new_prop("tag", tag))
                }
                src_nodes.nodes_mut().push(n);
                self.save_document()?;
            }
            SourceNode::File(file_src) => {
                let mut n = kdl::KdlNode::new("file");
                n.push(kdl::KdlEntry::new(
                    file_src.bundle_path.to_string_lossy().to_string(),
                ));
                if let Some(target_path) = file_src.target_path {
                    n.push(kdl::KdlEntry::new(
                        target_path.to_string_lossy().to_string(),
                    ));
                }
                src_nodes.nodes_mut().push(n);
                self.save_document()?;
            }
            SourceNode::Patch(patch_src) => {
                let mut n = kdl::KdlNode::new("patch");
                n.push(kdl::KdlEntry::new(
                    patch_src.bundle_path.to_string_lossy().to_string(),
                ));
                if let Some(dir_to_drop) = patch_src.drop_directories {
                    n.push(kdl::KdlEntry::new_prop("drop-directories", dir_to_drop));
                }
                src_nodes.nodes_mut().push(n);
                self.save_document()?;
            }
            SourceNode::Overlay(overlay_src) => {
                let mut n = kdl::KdlNode::new("overlay");
                n.push(kdl::KdlEntry::new(
                    overlay_src.bundle_path.to_string_lossy().to_string(),
                ));
                src_nodes.nodes_mut().push(n);
                self.save_document()?;
            }
        }
        Ok(())
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_name(&self) -> String {
        self.package_document.name.clone()
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

    #[knuffel(property)]
    pub sha512: String,
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

impl FileSource {
    pub fn new<P: AsRef<Path>>(bundle_path: P, target_path: Option<P>) -> BundleResult<Self> {
        Ok(Self {
            bundle_path: bundle_path.as_ref().to_path_buf(),
            target_path: target_path.as_ref().map(|p| p.as_ref().to_path_buf()),
        })
    }
}

#[derive(knuffel::Decode)]
pub struct PatchSource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
    #[knuffel(property)]
    pub drop_directories: Option<i64>,
}

impl PatchSource {
    pub fn new<P: AsRef<Path>>(
        bundle_path: P,
        drop_directories: Option<i64>,
    ) -> BundleResult<Self> {
        Ok(Self {
            bundle_path: bundle_path.as_ref().to_path_buf(),
            drop_directories,
        })
    }
}

#[derive(knuffel::Decode)]
pub struct OverlaySource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
}

impl OverlaySource {
    pub fn new<P: AsRef<Path>>(bundle_path: P) -> BundleResult<Self> {
        Ok(Self {
            bundle_path: bundle_path.as_ref().to_path_buf(),
        })
    }
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
        let _b = match knuffel::parse::<Package>("openssl", &package_document_string) {
            Ok(b) => b,
            Err(e) => {
                assert!(false);
                panic!("{:?}", miette::Report::new(e));
            }
        };
    }
}
