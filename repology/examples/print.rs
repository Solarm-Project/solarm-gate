use miette::IntoDiagnostic;
use semver::Version;
use serde_json::to_string_pretty;
use repology::*;

fn main() -> miette::Result<()> {
	let data = 
        MetadataBuilder::default()
            .summary("ansible - Radically simple IT automation")
            .source_name("python/ansible")
            .fmri("library/python/ansible@7.4.0,5.11-2023.0.0.1:20230421T131743Z")
            .project_name("ansible")
            .homepages([String::from("https://ansible.com/")])
            .licenses([String::from("GPL-3.0-only")])
            .version(Version::parse("7.4.0").into_diagnostic()?)
            .source_links([String::from("https://files.pythonhosted.org/packages/45/4b/2087a0fe8265828df067e57d7d156426cdc8f7cd94ad3178c6510d81e2c0/ansible-7.4.0.tar.gz")])
            .categories([String::from("Development/Python")])
            .build()?;
	let s = to_string_pretty(&data).into_diagnostic()?;
	println!("{}", &s);
	Ok(())
}
