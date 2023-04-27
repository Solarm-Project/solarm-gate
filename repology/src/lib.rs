use derive_builder::Builder;
use miette::Diagnostic;
use semver::Version;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    UninitializedFieldError(#[from] derive_builder::UninitializedFieldError),

    #[error(transparent)]
    SemVerError(#[from] semver::Error),
}

#[derive(Debug, Default, PartialEq, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SupportedArchitectures {
    #[default]
    AMD64,
    ARM64,
    SPARC64,
}

#[derive(Debug, Serialize, Builder, PartialEq)]
#[builder(setter(into, strip_option), build_fn(error = "self::Error"))]
pub struct Metadata {
    #[builder(default)]
    pub maintainers: Vec<String>,
    pub summary: String,
    pub source_name: String,
    pub fmri: String,
    pub project_name: String,
    #[builder(default)]
    pub arch: SupportedArchitectures,
    pub homepages: Vec<String>,
    #[builder(default)]
    pub licenses: Vec<String>,
    pub source_links: Vec<String>,
    pub categories: Vec<String>,
    pub version: Version,
}

impl MetadataBuilder {
    pub fn add_maintainer<S: Into<String>>(&mut self, maintainer: S) -> &mut Self {
        if let Some(maintainers) = self.maintainers.as_mut() {
            maintainers.push(maintainer.into());
        } else {
            self.maintainers = Some(vec![maintainer.into()]);
        }
        self
    }

    pub fn add_homepage<S: Into<String>>(&mut self, value: S) -> &mut Self {
        if let Some(homepages) = self.homepages.as_mut() {
            homepages.push(value.into());
        } else {
            self.homepages = Some(vec![value.into()]);
        }
        self
    }

    pub fn add_license<S: Into<String>>(&mut self, value: S) -> &mut Self {
        if let Some(licenses) = self.licenses.as_mut() {
            licenses.push(value.into());
        } else {
            self.licenses = Some(vec![value.into()]);
        }
        self
    }

    pub fn add_category<S: Into<String>>(&mut self, value: S) -> &mut Self {
        if let Some(categories) = self.categories.as_mut() {
            categories.push(value.into());
        } else {
            self.categories = Some(vec![value.into()]);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    type Result<T> = miette::Result<T, Error>;
    use super::*;
    use expectorate::assert_contents;
    use miette::IntoDiagnostic;

    fn build_sample() -> Result<Metadata> {
        MetadataBuilder::default()
            .summary("ansible - Radically simple IT automation")
            .source_name("python/ansible")
            .fmri("library/python/ansible@7.4.0,5.11-2023.0.0.1:20230421T131743Z")
            .project_name("ansible")
            .homepages([String::from("https://ansible.com/")])
            .licenses([String::from("GPL-3.0-only")])
            .version(Version::parse("7.4.0")?)
            .source_links([String::from("https://files.pythonhosted.org/packages/45/4b/2087a0fe8265828df067e57d7d156426cdc8f7cd94ad3178c6510d81e2c0/ansible-7.4.0.tar.gz")])
            .categories([String::from("Development/Python")])
            .build()
    }

    #[test]
    fn it_works() -> Result<()> {
        let actual = build_sample()?;
        assert_eq!("7.4.0", actual.version.to_string().as_str());
        assert_eq!(
            "ansible - Radically simple IT automation",
            actual.summary.as_str()
        );
        assert_eq!("python/ansible", actual.source_name.as_str());
        Ok(())
    }

    #[test]
    fn serialize_test() -> miette::Result<()> {
        let sample = build_sample()?;
        let actual = serde_json::to_string_pretty(&sample).into_diagnostic()?;
        println!("{}", &actual);
        assert_contents("ansible_repology_data.json", &actual);
        Ok(())
    }
}
