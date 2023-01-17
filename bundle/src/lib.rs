use kdl::KdlDocument;
use miette::{Diagnostic, IntoDiagnostic};
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
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
    #[error("unknown build type {0}")]
    UnknownBuildType(String),
}

type BundleResult<T> = std::result::Result<T, BundleError>;

#[derive(Debug)]
pub struct Bundle {
    path: PathBuf,
    pub package_document: Package,
    kdl_doc: Option<KdlDocument>,
}

impl Bundle {
    pub fn open_local<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
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

        if kdl_doc.get("source").is_none() {
            kdl_doc.nodes_mut().push(kdl::KdlNode::new("source"))
        }

        let src_node: &mut kdl::KdlNode = kdl_doc.get_mut("source").unwrap();
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
    #[knuffel(child, unwrap(argument))]
    pub classification: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub summary: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub license_file: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub license: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub prefix: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub version: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub revision: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub project_url: Option<String>,
    #[knuffel(children(name = "source"))]
    pub sources: Vec<SourceSection>,
    #[knuffel(child)]
    build: Option<BuildSection>,
    #[knuffel(children(name = "dependency"))]
    pub dependencies: Vec<Dependency>,
    #[knuffel(children(name = "transform"))]
    pub transforms: Vec<Transform>,
}

impl Package {
    pub fn to_document(&self) -> kdl::KdlDocument {
        let pkg_node = self.to_node();
        pkg_node
            .children()
            .unwrap_or(&kdl::KdlDocument::new())
            .clone()
    }

    pub fn get_build_section(&self) -> Option<BuildSection> {
        self.build.clone()
    }

    pub fn ensure_build_section(&self) -> BuildSection {
        self.build.clone().unwrap()
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("package");
        let doc = node.ensure_children();
        let mut name_node = kdl::KdlNode::new("name");
        name_node.insert(0, self.name.as_str());
        doc.nodes_mut().push(name_node);

        if let Some(classification) = &self.classification {
            let mut classification_node = kdl::KdlNode::new("classification");
            classification_node.insert(0, classification.as_str());
            doc.nodes_mut().push(classification_node);
        }

        if let Some(summary) = &self.summary {
            let mut summary_node = kdl::KdlNode::new("summary");
            summary_node.insert(0, summary.as_str());
            doc.nodes_mut().push(summary_node);
        }

        if let Some(license_file) = &self.license_file {
            let mut license_file_node = kdl::KdlNode::new("license-file");
            license_file_node.insert(0, license_file.as_str());
            doc.nodes_mut().push(license_file_node);
        }

        if let Some(license) = &self.license {
            let mut license_node = kdl::KdlNode::new("license");
            license_node.insert(0, license.as_str());
            doc.nodes_mut().push(license_node);
        }

        if let Some(prefix) = &self.prefix {
            let mut prefix_node = kdl::KdlNode::new("prefix");
            prefix_node.insert(0, prefix.as_str());
            doc.nodes_mut().push(prefix_node);
        }

        if let Some(version) = &self.version {
            let mut version_node = kdl::KdlNode::new("version");
            version_node.insert(0, version.as_str());
            doc.nodes_mut().push(version_node);
        }

        if let Some(revision) = &self.revision {
            let mut revision_node = kdl::KdlNode::new("revision");
            revision_node.insert(0, revision.as_str());
            doc.nodes_mut().push(revision_node);
        }

        if let Some(project_url) = &self.project_url {
            let mut project_url_node = kdl::KdlNode::new("project-url");
            project_url_node.insert(0, project_url.as_str());
            doc.nodes_mut().push(project_url_node);
        }

        if self.sources.len() > 0 {
            for src in &self.sources {
                let source_node = src.to_node();
                doc.nodes_mut().push(source_node);
            }
        }

        if let Some(build) = &self.build {
            let build_node = build.to_node();
            doc.nodes_mut().push(build_node);
        }

        for dependency in &self.dependencies {
            let dep_node = dependency.to_node();
            doc.nodes_mut().push(dep_node);
        }

        for tr in &self.transforms {
            let tr_node = tr.to_node();
            doc.nodes_mut().push(tr_node);
        }

        node
    }

    pub fn merge_into_mut(&mut self, other: &Package) {
        if let Some(classification) = &other.classification {
            self.classification = Some(classification.clone());
        }

        if let Some(summary) = &other.summary {
            self.summary = Some(summary.clone());
        }

        if let Some(license_file) = &other.license_file {
            self.license_file = Some(license_file.clone());
        }

        if let Some(license) = &other.license {
            self.license = Some(license.clone());
        }

        if let Some(prefix) = &other.prefix {
            self.prefix = Some(prefix.clone());
        }

        if let Some(version) = &other.version {
            self.version = Some(version.clone());
        }

        if let Some(revision) = &other.revision {
            self.revision = Some(revision.clone());
        }

        if let Some(project_url) = &other.project_url {
            self.project_url = Some(project_url.clone());
        }

        if let Some(build_section) = &other.build {
            let mut self_build = self.ensure_build_section();
            for opt in &build_section.options {
                self_build.options.push(opt.clone());
            }
            for flag in &build_section.flags {
                self_build.flags.push(flag.clone());
            }
            for script in &build_section.scripts {
                self_build.scripts.push(script.clone());
            }

            if let Some(sysroot) = &build_section.package_sysroot {
                self_build.package_sysroot = Some(sysroot.clone());
            }

            self.build = Some(self_build.clone());
        }

        for src in &other.sources {
            self.sources.push(src.clone());
        }

        for dep in &other.dependencies {
            self.dependencies.push(dep.clone());
        }

        for transform in &other.transforms {
            self.transforms.push(transform.clone());
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Transform {
    #[knuffel(argument)]
    pub rule: String,
}

impl Transform {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("transform");
        node.insert(0, self.rule.as_str());
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Dependency {
    #[knuffel(argument)]
    pub name: String,
}

impl Dependency {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("dependency");
        node.insert(0, self.name.as_str());
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct SourceSection {
    #[knuffel(argument)]
    pub name: Option<String>,
    #[knuffel(children)]
    pub sources: Vec<SourceNode>,
}

impl SourceSection {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut source_node = kdl::KdlNode::new("source");
        if let Some(name) = &self.name {
            source_node.insert(0, name.as_str());
        }

        for src in &self.sources {
            let src_node = match src {
                SourceNode::Archive(s) => s.to_node(),
                SourceNode::Git(s) => s.to_node(),
                SourceNode::File(s) => s.to_node(),
                SourceNode::Patch(s) => s.to_node(),
                SourceNode::Overlay(s) => s.to_node(),
            };
            let doc = source_node.ensure_children();
            doc.nodes_mut().push(src_node);
        }

        source_node
    }
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

impl ArchiveSource {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("archive");
        node.insert(0, self.src.as_str());
        node.insert("sha512", self.sha512.as_str());
        node
    }
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
    #[knuffel(property)]
    pub must_stay_as_repo: Option<bool>,
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

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("git");
        node.insert(0, self.repository.as_str());
        if let Some(branch) = &self.branch {
            node.insert("branch", branch.as_str());
        }
        if let Some(tag) = &self.tag {
            node.insert("tag", tag.as_str());
        }
        if let Some(archive) = self.archive.clone() {
            node.insert("archive", archive);
        }
        if let Some(must_stay_as_repo) = self.must_stay_as_repo.clone() {
            node.insert("must-stay-as-repo", must_stay_as_repo);
        }
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct FileSource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
    #[knuffel(argument)]
    target_path: Option<PathBuf>,
}

impl FileSource {
    pub fn new<P: AsRef<Path>>(bundle_path: P, target_path: Option<P>) -> BundleResult<Self> {
        Ok(Self {
            bundle_path: bundle_path.as_ref().to_path_buf(),
            target_path: target_path.as_ref().map(|p| p.as_ref().to_path_buf()),
        })
    }

    pub fn get_bundle_path<P: AsRef<Path>>(&self, base_path: P) -> PathBuf {
        base_path.as_ref().join(&self.bundle_path)
    }

    pub fn get_target_path(&self) -> PathBuf {
        if let Some(p) = &self.target_path {
            p.clone()
        } else {
            self.bundle_path.clone()
        }
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("file");
        node.insert(0, self.bundle_path.to_string_lossy().to_string().as_str());
        if let Some(target_path) = &self.target_path {
            node.insert(1, target_path.to_string_lossy().to_string().as_str());
        }
        node
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

    pub fn get_bundle_path<P: AsRef<Path>>(&self, base_path: P) -> PathBuf {
        base_path.as_ref().join(&self.bundle_path)
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("patch");
        node.insert(0, self.bundle_path.to_string_lossy().to_string().as_str());
        if let Some(dirs) = self.drop_directories.clone() {
            node.insert("drop-directories", dirs);
        }
        node
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

    pub fn get_bundle_path<P: AsRef<Path>>(&self, base_path: P) -> PathBuf {
        base_path.as_ref().join(&self.bundle_path)
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("overlay");
        node.insert(0, self.bundle_path.to_string_lossy().to_string().as_str());
        node
    }
}

#[derive(Debug, Default, knuffel::Decode, Clone)]
pub struct BuildSection {
    #[knuffel(argument, default, str)]
    pub build_type: BuildType,
    #[knuffel(children(name = "option"))]
    pub options: Vec<BuildOptionNode>,
    #[knuffel(children(name = "flag"))]
    pub flags: Vec<BuildFlagNode>,
    #[knuffel(children(name = "script"))]
    pub scripts: Vec<ScriptNode>,
    #[knuffel(child)]
    pub package_sysroot: Option<SysRootNode>,
}

#[derive(Debug, Default, knuffel::Decode, Clone)]
pub struct SysRootNode {
    #[knuffel(property)]
    pub from: String,
    #[knuffel(property)]
    pub target: String,
    #[knuffel(property)]
    pub name: String,
}

impl SysRootNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("package-sysroot");
        node.insert("from", self.from.as_str());
        node.insert("target", self.target.as_str());
        node.insert("name", self.name.as_str());
        node
    }
}

#[derive(Debug, Default, knuffel::Decode, Clone)]
pub struct ScriptNode {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(property)]
    pub prototype_dir: PathBuf,
}

impl ScriptNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("script");
        node.insert(0, self.name.as_str());
        node.insert(
            "prototype-dir",
            self.prototype_dir.to_string_lossy().to_string().as_str(),
        );
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub enum BuildType {
    Configure,
    CMake,
    Meson,
    Custom,
}

impl Default for BuildType {
    fn default() -> Self {
        Self::Configure
    }
}

impl FromStr for BuildType {
    type Err = BundleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "configure" => Ok(BuildType::Configure),
            "meson" => Ok(BuildType::Meson),
            "cmake" => Ok(BuildType::CMake),
            "custom" => Ok(BuildType::Custom),
            x => Err(BundleError::UnknownBuildType(x.to_string())),
        }
    }
}

impl ToString for BuildType {
    fn to_string(&self) -> String {
        match self {
            BuildType::Configure => String::from("configure"),
            BuildType::CMake => String::from("cmake"),
            BuildType::Meson => String::from("meson"),
            BuildType::Custom => String::from("custom"),
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct BuildFlagNode {
    #[knuffel(argument)]
    pub flag: String,
}

impl BuildFlagNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("flag");
        node.insert(0, self.flag.as_str());
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct BuildOptionNode {
    #[knuffel(argument)]
    pub option: String,
}

impl BuildOptionNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("option");
        node.insert(0, self.option.as_str());
        node
    }
}

impl BuildSection {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("build");
        node.insert(0, self.build_type.to_string().as_str());
        let doc = node.ensure_children();

        for option in &self.options {
            doc.nodes_mut().push(option.to_node());
        }

        for flag in &self.flags {
            doc.nodes_mut().push(flag.to_node());
        }

        for script in &self.scripts {
            doc.nodes_mut().push(script.to_node());
        }

        if let Some(sysroot) = &self.package_sysroot {
            doc.nodes_mut().push(sysroot.to_node());
        }

        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct FileNode {
    #[knuffel(child, unwrap(argument))]
    pub include: String,
}

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
            .map(|path| Bundle::open_local(&path))
            .collect::<miette::Result<Vec<Bundle>>>()?;
        for bundle in bundles {
            assert_ne!(bundle.package_document.name, String::from(""))
        }

        Ok(())
    }

    #[test]
    fn parse_openssl() -> miette::Result<()> {
        let bundle_path = Path::new("../packages/openssl");
        let _b = Bundle::open_local(bundle_path)?;

        Ok(())
    }

    #[test]
    fn parse_binutils_gdb() -> miette::Result<()> {
        let bundle_path = Path::new("../packages/binutils-gdb");
        let _b = Bundle::open_local(bundle_path)?;

        Ok(())
    }
}
