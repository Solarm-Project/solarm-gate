use bonsaidb::core::schema::{Collection, Schema};
use serde::{Deserialize, Serialize};

pub const DATABASE_NAME: &str = "forge";

#[derive(Debug, Schema)]
#[schema(name = "forge", collections = [Publisher, Profile])]
pub struct ForgeSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "publishers")]
pub struct Publisher {
    pub name: String,
    pub public: bool,
    #[serde(default)]
    pub owners: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitHubToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "profiles")]
pub struct Profile {
    pub username: String,
    pub token: Option<GitHubToken>,
    #[serde(default)]
    pub ssh_pub_keys: Vec<String>,
    #[serde(default)]
    pub gpg_pub_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "packages")]
pub struct Package {
    pub name: String,
    pub publisher: String,
    #[serde(default)]
    pub manifests: Vec<String>,
    pub document: bundle::Package,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "gates")]
pub struct Gate {
    pub name: String,
    pub gate_doc: gate::Gate,
    pub publisher_ref: i64,
    pub packages: Vec<i64>,
}
