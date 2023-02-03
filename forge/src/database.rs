use bonsaidb::core::key::{Key, KeyEncoding};
use bonsaidb::core::schema::{Collection, Schema};
use serde::{Deserialize, Serialize};

pub const DATABASE_NAME: &str = "forge";

#[derive(Debug, Schema)]
#[schema(name = "forge", collections = [Publisher, Profile])]
pub struct ForgeSchema;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct PrimaryKey(uuid::Uuid);

impl<'k> Key<'k> for PrimaryKey {
    fn from_ord_bytes(bytes: &'k [u8]) -> Result<Self, Self::Error> {
        Ok(Self(uuid::Uuid::from_slice(bytes)?))
    }

    fn next_value(&self) -> Result<Self, bonsaidb::core::key::NextValueError> {
        Ok(PrimaryKey::new())
    }

    fn first_value() -> Result<Self, bonsaidb::core::key::NextValueError> {
        Ok(PrimaryKey::new())
    }
}

impl<'k> KeyEncoding<'k, Self> for PrimaryKey {
    type Error = uuid::Error;

    const LENGTH: Option<usize> = None;

    fn as_ord_bytes(&'k self) -> Result<std::borrow::Cow<'k, [u8]>, Self::Error> {
        Ok(std::borrow::Cow::Borrowed(self.0.as_bytes()))
    }
}

impl PrimaryKey {
    pub fn new() -> Self {
        PrimaryKey(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "publishers", primary_key=PrimaryKey)]
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
#[collection(name="profiles", primary_key=PrimaryKey)]
pub struct Profile {
    pub username: String,
    pub token: Option<GitHubToken>,
    #[serde(default)]
    pub ssh_pub_keys: Vec<String>,
    #[serde(default)]
    pub gpg_pub_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name="packages", primary_key=PrimaryKey)]
pub struct Package {
    pub name: String,
    pub publisher: String,
    #[serde(default)]
    pub manifests: Vec<String>,
    pub document: bundle::Package,
}
