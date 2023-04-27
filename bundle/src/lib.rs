use derive_builder::Builder;
use kdl::KdlValue;
use miette::{Diagnostic, IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};
use std::{
    fs::{read_to_string, File},
    io::Write,
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
    #[diagnostic(code(bundle::kdl_error))]
    Kdl(#[from] kdl::KdlError),
    #[error(transparent)]
    #[diagnostic(code(bundle::url_parse_error))]
    UrlParseError(#[from] url::ParseError),
    #[error("unknown build type {0}")]
    UnknownBuildType(String),
    #[error("build types {0} and {1} are not mergeable")]
    NonMergableBuildSections(String, String),
    #[error(transparent)]
    UninitializedFieldError(#[from] derive_builder::UninitializedFieldError),
}

type BundleResult<T> = std::result::Result<T, BundleError>;

#[derive(Debug)]
pub struct Bundle {
    path: PathBuf,
    pub package_document: Package,
}

impl Bundle {
    pub fn open_local<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = path
            .as_ref()
            .canonicalize()
            .into_diagnostic()
            .wrap_err(miette::miette!("could not open bundle"))?;

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
            })
        } else {
            let package_document = knuffel::parse::<Package>(&name, &package_document_string)?;
            Ok(Self {
                path,
                package_document,
            })
        }
    }

    fn open_document(&mut self) -> miette::Result<()> {
        let data_string = read_to_string(&self.path.join("package.kdl"))
            .into_diagnostic()
            .wrap_err("could not open package document")?;
        self.package_document = knuffel::parse::<Package>("package.kdl", &data_string)?;
        Ok(())
    }

    fn save_document(&self) -> BundleResult<()> {
        let doc_str = self.package_document.to_document().to_string();
        let mut f = File::create(&self.path.join("package.kdl"))?;
        f.write_all(doc_str.as_bytes())?;
        Ok(())
    }

    pub fn add_source(&mut self, node: SourceNode) -> miette::Result<()> {
        if let Some(src_section) = self.package_document.sources.first_mut() {
            src_section.sources.push(node);
        } else {
            let src_section = SourceSection {
                name: None,
                sources: vec![node],
            };
            self.package_document.sources.push(src_section);
        };
        self.save_document()?;
        self.open_document()?;
        Ok(())
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_name(&self) -> String {
        self.package_document.name.clone()
    }

    pub fn get_mogrify_manifest(&self) -> Option<PathBuf> {
        let file_path = self.path.join("manifest.mog");
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize, Builder)]
#[builder(setter(into, strip_option), build_fn(error = "self::BundleError"))]
pub struct Package {
    #[knuffel(child, unwrap(argument))]
    pub name: String,

    #[knuffel(child, unwrap(argument))]
    pub project_name: String,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub classification: Option<String>,

    #[knuffel(children(name = "maintainer"), unwrap(argument))]
    #[builder(default)]
    pub maintainers: Vec<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub summary: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub license_file: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub license: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub prefix: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub version: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub revision: Option<String>,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub project_url: Option<String>,

    #[knuffel(child)]
    #[builder(default)]
    pub seperate_build_dir: bool,

    #[knuffel(child, unwrap(argument))]
    #[builder(default)]
    pub maintainer: Option<String>,

    #[knuffel(children(name = "source"))]
    #[builder(default)]
    pub sources: Vec<SourceSection>,

    #[knuffel(children(name = "dependency"))]
    #[builder(default)]
    pub dependencies: Vec<Dependency>,

    #[knuffel(children(name = "build"))]
    #[builder(default)]
    build_section: Vec<BuildSection>,
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
        self.build_section.first().map(|b| b.clone())
    }

    pub fn ensure_build_section(&self) -> BuildSection {
        self.build_section
            .first()
            .map(|b| b.clone())
            .unwrap_or(BuildSection::default())
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("package");
        let doc = node.ensure_children();
        let mut name_node = kdl::KdlNode::new("name");
        name_node.insert(0, self.name.as_str());
        doc.nodes_mut().push(name_node);

        let mut project_name_node = kdl::KdlNode::new("project-name");
        project_name_node.insert(0, self.name.as_str());
        doc.nodes_mut().push(project_name_node);

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

        if let Some(maintainer) = &self.maintainer {
            let mut maintainer_node = kdl::KdlNode::new("maintainer");
            maintainer_node.insert(0, maintainer.as_str());
            doc.nodes_mut().push(maintainer_node);
        }

        if self.sources.len() > 0 {
            for src in &self.sources {
                let source_node = src.to_node();
                doc.nodes_mut().push(source_node);
            }
        }

        if let Some(build) = &self.get_build_section() {
            let build_node = build.to_node();
            doc.nodes_mut().push(build_node);
        }

        for dependency in &self.dependencies {
            let dep_node = dependency.to_node();
            doc.nodes_mut().push(dep_node);
        }

        node
    }

    pub fn merge_into_mut(&mut self, other: &Package) -> BundleResult<()> {
        self.name = other.name.clone();

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

        if let Some(maintainer) = &other.maintainer {
            self.maintainer = Some(maintainer.clone());
        }

        if let Some(build_section) = &other.get_build_section() {
            let self_build = self.ensure_build_section();
            let final_build = match build_section {
                BuildSection::Configure(other_configure) => match self_build {
                    BuildSection::Configure(c) => {
                        Ok(BuildSection::Configure(ConfigureBuildSection {
                            options: c
                                .options
                                .into_iter()
                                .chain(other_configure.options.clone())
                                .collect(),
                            flags: c
                                .flags
                                .into_iter()
                                .chain(other_configure.flags.clone())
                                .collect(),
                            compiler: if let Some(compiler) = &other_configure.compiler {
                                Some(compiler.clone())
                            } else {
                                c.compiler
                            },
                            linker: if let Some(linker) = &other_configure.linker {
                                Some(linker.clone())
                            } else {
                                c.linker
                            },
                        }))
                    }
                    BuildSection::NoBuild => Ok(BuildSection::Configure(other_configure.clone())),
                    x => Err(BundleError::NonMergableBuildSections(
                        x.to_string(),
                        build_section.clone().to_string(),
                    )),
                },
                BuildSection::CMake => todo!(),
                BuildSection::Meson => todo!(),
                BuildSection::Build(other_scripts) => match self_build {
                    BuildSection::Build(s) => Ok(BuildSection::Build(ScriptBuildSection {
                        scripts: s
                            .scripts
                            .into_iter()
                            .chain(other_scripts.scripts.clone())
                            .collect(),
                        install_directives: s
                            .install_directives
                            .into_iter()
                            .chain(other_scripts.install_directives.clone())
                            .collect(),
                    })),
                    BuildSection::NoBuild => Ok(BuildSection::Build(other_scripts.clone())),
                    x => Err(BundleError::NonMergableBuildSections(
                        x.to_string(),
                        build_section.to_string(),
                    )),
                },
                BuildSection::NoBuild => Ok(BuildSection::NoBuild),
            }?;

            self.build_section = vec![final_build];
        }

        for src in &other.sources {
            self.sources.push(src.clone());
        }

        for dep in &other.dependencies {
            self.dependencies.push(dep.clone());
        }

        Ok(())
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct Dependency {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(property, default = false)]
    pub dev: bool,
    #[knuffel(property)]
    pub kind: Option<DependencyKind>,
}

impl Dependency {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("dependency");
        node.insert(0, self.name.as_str());

        if self.dev {
            node.insert("dev", true);
        }

        if let Some(kind) = &self.kind {
            node.insert("kind", kind);
        }

        node
    }
}

#[derive(Debug, knuffel::DecodeScalar, Clone, Serialize, Deserialize)]
pub enum DependencyKind {
    Require,
    Incorporate,
    Optional,
}

impl From<&DependencyKind> for KdlValue {
    fn from(value: &DependencyKind) -> Self {
        match value {
            DependencyKind::Require => "require".into(),
            DependencyKind::Incorporate => "incorporate".into(),
            DependencyKind::Optional => "optional".into(),
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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
                SourceNode::Directory(s) => s.to_node(),
            };
            let doc = source_node.ensure_children();
            doc.nodes_mut().push(src_node);
        }

        source_node
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
pub enum SourceNode {
    Archive(ArchiveSource),
    Git(GitSource),
    File(FileSource),
    Directory(DirectorySource),
    Patch(PatchSource),
    Overlay(OverlaySource),
}

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct ArchiveSource {
    #[knuffel(argument)]
    pub src: String,

    #[knuffel(property)]
    pub sha512: Option<String>,

    #[knuffel(property)]
    pub sha256: Option<String>,

    #[knuffel(property)]
    pub signature_url_extension: Option<String>,

    #[knuffel(property)]
    pub signature_url: Option<String>,
}

impl ArchiveSource {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("archive");
        node.insert(0, self.src.as_str());
        if let Some(sha512) = &self.sha512 {
            node.insert("sha512", sha512.as_str());
        }
        if let Some(sha256) = &self.sha256 {
            node.insert("sha256", sha256.as_str());
        }
        if let Some(signature_ext) = &self.signature_url_extension {
            node.insert("singature-url-extension", signature_ext.as_str());
        }
        if let Some(sig_url) = &self.signature_url {
            node.insert("signature-url", sig_url.as_str());
        }
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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

    // Directory where to unpack sources into the first git source can ignore this on the second it is required
    #[knuffel(property)]
    pub directory: Option<String>,
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

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct DirectorySource {
    #[knuffel(argument)]
    bundle_path: PathBuf,
    #[knuffel(argument)]
    target_path: Option<PathBuf>,
}

impl DirectorySource {
    pub fn new<P: AsRef<Path>>(bundle_path: P, target_path: Option<P>) -> BundleResult<Self> {
        Ok(Self {
            bundle_path: bundle_path.as_ref().to_path_buf(),
            target_path: target_path.as_ref().map(|p| p.as_ref().to_path_buf()),
        })
    }

    pub fn get_bundle_path<P: AsRef<Path>>(&self, base_path: P) -> PathBuf {
        base_path.as_ref().join(&self.bundle_path)
    }

    pub fn get_name(&self) -> String {
        self.bundle_path.display().to_string()
    }

    pub fn get_target_path(&self) -> PathBuf {
        if let Some(p) = &self.target_path {
            p.clone()
        } else {
            self.bundle_path.clone()
        }
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("directory");
        node.insert(0, self.bundle_path.to_string_lossy().to_string().as_str());
        if let Some(target_path) = &self.target_path {
            node.insert(1, target_path.to_string_lossy().to_string().as_str());
        }
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub enum BuildSection {
    Configure(ConfigureBuildSection),
    CMake,
    Meson,
    Build(ScriptBuildSection),
    #[default]
    NoBuild,
}

impl ToString for BuildSection {
    fn to_string(&self) -> String {
        match &self {
            BuildSection::Configure(_) => "configure",
            BuildSection::CMake => "cmake",
            BuildSection::Meson => "meson",
            BuildSection::Build(_) => "build",
            BuildSection::NoBuild => "no-build",
        }
        .to_string()
    }
}

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct ConfigureBuildSection {
    #[knuffel(children(name = "option"))]
    pub options: Vec<BuildOptionNode>,
    #[knuffel(children(name = "flag"))]
    pub flags: Vec<BuildFlagNode>,
    #[knuffel(child, unwrap(argument))]
    pub compiler: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub linker: Option<String>,
}

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct ScriptBuildSection {
    #[knuffel(children(name = "script"))]
    pub scripts: Vec<ScriptNode>,
    #[knuffel(children(name = "install"))]
    pub install_directives: Vec<InstallDirectiveNode>,
}

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct InstallDirectiveNode {
    #[knuffel(property)]
    pub src: String,
    #[knuffel(property)]
    pub target: String,
    #[knuffel(property)]
    pub name: String,
    #[knuffel(property)]
    pub pattern: Option<String>,
    #[knuffel(property(name = "match"))]
    pub fmatch: Option<String>,
}

impl InstallDirectiveNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("package-directory");
        node.insert("src", self.src.as_str());
        node.insert("target", self.target.as_str());
        node.insert("name", self.name.as_str());
        node
    }
}

#[derive(Debug, Default, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct ScriptNode {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(property)]
    pub prototype_dir: Option<PathBuf>,
}

impl ScriptNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("script");
        node.insert(0, self.name.as_str());
        if let Some(prototype_dir) = &self.prototype_dir {
            node.insert(
                "prototype-dir",
                prototype_dir.to_string_lossy().to_string().as_str(),
            );
        }
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
pub struct BuildFlagNode {
    #[knuffel(argument)]
    pub flag: String,
    #[knuffel(property(name = "name"))]
    pub flag_name: Option<String>,
}

impl BuildFlagNode {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("flag");
        node.insert(0, self.flag.as_str());
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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
        match &self {
            BuildSection::Configure(c) => {
                let mut node = kdl::KdlNode::new("configure");
                let doc = node.ensure_children();
                for option in &c.options {
                    doc.nodes_mut().push(option.to_node());
                }

                for flag in &c.flags {
                    doc.nodes_mut().push(flag.to_node());
                }

                if let Some(compiler) = &c.compiler {
                    let mut n = kdl::KdlNode::new("compiler");
                    n.insert(0, compiler.clone());
                    doc.nodes_mut().push(n);
                }

                if let Some(linker) = &c.linker {
                    let mut n = kdl::KdlNode::new("linker");
                    n.insert(0, linker.clone());
                    doc.nodes_mut().push(n);
                }

                node
            }
            BuildSection::CMake => todo!(),
            BuildSection::Meson => todo!(),
            BuildSection::Build(s) => {
                let mut node = kdl::KdlNode::new("build");
                let doc = node.ensure_children();
                for script in &s.scripts {
                    doc.nodes_mut().push(script.to_node());
                }

                for package_directory in &s.install_directives {
                    doc.nodes_mut().push(package_directory.to_node());
                }

                node
            }
            BuildSection::NoBuild => kdl::KdlNode::new("no-build"),
        }
    }
}

#[derive(Debug, knuffel::Decode, Clone, Serialize, Deserialize)]
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
