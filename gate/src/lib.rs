use bundle::Package;
use miette::{Diagnostic, IntoDiagnostic};
use std::{
    fs::{read_to_string, File},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum GateError {
    #[error(transparent)]
    #[diagnostic(code(gate::io))]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(code(bundle::kdl_error))]
    Kdl(#[from] kdl::KdlError),
    #[error(transparent)]
    #[diagnostic(code(bundle::url_parse_error))]
    UrlParseError(#[from] url::ParseError),
    #[error("the path {0} cannot be opened as gate file plese provide the correct path to a gate.kdl file")]
    NoFileNameError(String),
    #[error("only one packages with name {0} should be present. There are {1}")]
    TooManyPackagesWithTheSameName(String, usize),
    #[error("no package with name {0}")]
    NoSuchPackage(String),
    #[error("distribution type {0} is not known use one of 'tarball', 'ips'")]
    UnknownDistributionType(String),
}

type GateResult<T> = std::result::Result<T, GateError>;

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Gate {
    path: PathBuf,
    #[knuffel(child, unwrap(argument))]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub version: String,
    #[knuffel(child, unwrap(argument))]
    pub branch: String,
    #[knuffel(child)]
    pub distribution: Option<Distribution>,
    #[knuffel(children(name = "package"))]
    packages: Vec<Package>,
    #[knuffel(children(name = "transform"))]
    pub default_transforms: Vec<Transform>,
    #[knuffel(child, unwrap(argument))]
    pub publisher: String,
}

impl Default for Gate {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            name: String::new(),
            version: String::from("0.5.11"),
            branch: String::from("2023.0.0"),
            distribution: None,
            packages: vec![],
            default_transforms: vec![],
            publisher: String::from("userland"),
        }
    }
}

impl Gate {
    pub fn new<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = if !path.as_ref().is_absolute() {
            path.as_ref().canonicalize().into_diagnostic()?
        } else {
            path.as_ref().to_path_buf()
        };

        let gate_document_contents = read_to_string(&path).into_diagnostic()?;
        let name = path
            .file_name()
            .ok_or(GateError::NoFileNameError(
                path.to_string_lossy().to_string(),
            ))?
            .to_string_lossy()
            .to_string();

        let mut gate = knuffel::parse::<Gate>(&name, &gate_document_contents)?;
        gate.path = path;
        Ok(gate)
    }

    pub fn get_package(&self, name: &str) -> Option<Package> {
        let gate_packages = self
            .packages
            .iter()
            .filter_map(|p| {
                if &p.name == name {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<Package>>();
        gate_packages.first().map(|p| p.clone())
    }

    pub fn to_document(&self) -> kdl::KdlDocument {
        let node = self.to_node();
        node.children().unwrap_or(&kdl::KdlDocument::new()).clone()
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("gate");
        let doc = node.ensure_children();
        let mut name_node = kdl::KdlNode::new("name");
        name_node.insert(0, self.name.as_str());
        doc.nodes_mut().push(name_node);

        let mut version_node = kdl::KdlNode::new("version");
        version_node.insert(0, self.version.as_str());
        doc.nodes_mut().push(version_node);

        let mut branch_node = kdl::KdlNode::new("branch");
        branch_node.insert(0, self.branch.as_str());
        doc.nodes_mut().push(branch_node);

        for pkg in &self.packages {
            let pkg_node = pkg.to_node();
            doc.nodes_mut().push(pkg_node);
        }

        if let Some(distribution) = &self.distribution {
            let distribution_node = distribution.to_node();
            doc.nodes_mut().push(distribution_node);
        }

        for tr in &self.default_transforms {
            let tr_node = tr.to_node();
            doc.nodes_mut().push(tr_node);
        }

        node
    }

    pub fn save(&self) -> GateResult<()> {
        let doc = self.to_document();
        let mut f = File::create(&self.path)?;
        f.write_all(doc.to_string().as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Transform {
    #[knuffel(arguments)]
    actions: Vec<String>,
    #[knuffel(property)]
    include: Option<String>,
}

impl Transform {
    pub fn to_string(&self) -> String {
        let mut lines = self.actions.clone();
        if let Some(include_prop) = &self.include {
            lines.push(format!("<include {}>", include_prop));
        }

        lines.join("\n")
    }

    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("transform");
        for (idx, action) in self.actions.iter().enumerate() {
            node.insert(idx, action.as_str());
        }

        if let Some(include_prop) = &self.include {
            node.insert("include", include_prop.as_str());
        }

        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub struct Distribution {
    #[knuffel(property(name = "type"), default, str)]
    pub distribution_type: DistributionType,
}

impl Distribution {
    pub fn to_node(&self) -> kdl::KdlNode {
        let mut node = kdl::KdlNode::new("distribution");
        let doc = node.ensure_children();
        let mut type_node = kdl::KdlNode::new("type");
        type_node.insert(0, self.distribution_type.to_string().as_str());
        doc.nodes_mut().push(type_node);
        node
    }
}

#[derive(Debug, knuffel::Decode, Clone)]
pub enum DistributionType {
    Tarbball,
    IPS,
}

impl Default for DistributionType {
    fn default() -> Self {
        Self::IPS
    }
}

impl FromStr for DistributionType {
    type Err = GateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tarball" | "tar" => Ok(Self::Tarbball),
            "ips" | "IPS" => Ok(Self::IPS),
            x => Err(GateError::UnknownDistributionType(x.to_string())),
        }
    }
}

impl ToString for DistributionType {
    fn to_string(&self) -> String {
        match self {
            DistributionType::Tarbball => String::from("tarball"),
            DistributionType::IPS => String::from("ips"),
        }
    }
}
