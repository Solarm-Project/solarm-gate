use kdl::KdlDocument;
use miette::{Diagnostic, IntoDiagnostic};
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
    #[diagnostic(code(bundle::kdl_error))]
    Kdl(#[from] kdl::KdlError),
    #[error(transparent)]
    #[diagnostic(code(bundle::url_parse_error))]
    UrlParseError(#[from] url::ParseError),
}

type BundleResult<T> = std::result::Result<T, BundleError>;

#[derive(Debug)]
pub struct Bundle {
    path: PathBuf,
    pub package_document: Package,
    kdl_doc: Option<KdlDocument>,
}

impl Bundle {
    pub fn new<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = path.as_ref().canonicalize().into_diagnostic()?;

        let (package_document_string, name) = if path.is_file() {
            (
                read_to_string(path.clone()).into_diagnostic()?,
                path.parent()
                    .ok_or(BundleError::NoPackageDocumentParentDir)?
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            (
                read_to_string(&path.join("package.kdl")).into_diagnostic()?,
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

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Package {
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(children)]
    pub sections: Vec<Section>,
}

#[derive(Debug, knuffel::Decode, Clone)]
pub enum Section {
    Sources(SourceSection),
    Build(BuildSection),
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct SourceSection {
    #[knuffel(children)]
    pub sources: Vec<SourceNode>,
}

#[derive(Debug, knuffel::Decode, Clone)]
pub enum SourceNode {
    Archive(ArchiveSource),
    Git(GitSource),
    File(FileSource),
    Patch(PatchSource),
    Overlay(OverlaySource),
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct ArchiveSource {
    #[knuffel(argument)]
    pub src: String,

    #[knuffel(property)]
    pub sha512: String,
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct GitSource {
    #[knuffel(argument)]
    pub repository: String,
    #[knuffel(property)]
    pub branch: Option<String>,
    #[knuffel(property)]
    pub tag: Option<String>,
    #[knuffel(property)]
    pub archive: Option<bool>,
}

impl GitSource {
    pub fn get_repo_prefix(&self) -> String {
        let repo_prefix_part = self
            .repository
            .rsplit_once('/')
            .unwrap_or(("", &self.repository))
            .1;
        let repo_prefix = if let Some(split_sucess) = repo_prefix_part.split_once('.') {
            split_sucess.0.to_string()
        } else {
            repo_prefix_part.to_string()
        };

        if let Some(tag) = &self.tag {
            format!("{}-{}", repo_prefix, tag)
        } else if let Some(branch) = &self.branch {
            format!("{}-{}", repo_prefix, branch)
        } else {
            format!("{}", repo_prefix)
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
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

#[derive(Debug, knuffel::Decode, Clone)]
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

#[derive(Debug, knuffel::Decode, Clone)]
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

#[derive(Debug, knuffel::Decode, Clone)]
pub struct BuildSection {}

#[cfg(test)]
mod tests {

    use miette::IntoDiagnostic;

    use crate::*;

    use std::path::{Path, PathBuf};

    /// Find all the bundle files at the given path. This will search the path
    /// recursively for any file named `package.kdl`.
    pub fn find_bundle_files(path: &Path) -> BundleResult<Vec<PathBuf>> {
        let mut result = Vec::new();
        find_bundle_files_rec(path, &mut result)?;
        Ok(result)
    }

    /// Search the file system recursively for all build files.
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
    fn test_read_all_samples() -> miette::Result<()> {
        let paths = find_bundle_files(Path::new("../packages")).into_diagnostic()?;
        let bundles = paths
            .into_iter()
            .map(|path| Bundle::new(&path))
            .collect::<miette::Result<Vec<Bundle>>>()?;
        for bundle in bundles {
            assert_ne!(bundle.package_document.name, String::from(""))
        }

        Ok(())
    }

    #[test]
    fn parse_one() -> miette::Result<()> {
        let bundle_path = Path::new("../packages/openssl/package.kdl");
        let package_document_string = read_to_string(&bundle_path).into_diagnostic()?;
        let _p = knuffel::parse::<Package>("openssl", &package_document_string)?;

        Ok(())
    }
}
