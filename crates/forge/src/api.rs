use crate::database;
use crate::database::Profile;
use crate::database::Publisher;
use async_graphql::{Context, InputObject, Object, Result, SimpleObject, Subscription};
use base64::Engine;
use bonsaidb::core::connection::AsyncStorageConnection;
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::core::transaction::Operation;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::AsyncDatabase;
use bonsaidb::local::AsyncStorage;
use bundle::Package as LibPackage;
use bundle::PackageBuilder;
use futures_util::Stream;
use graphql_client::{GraphQLQuery, Response};
use reqwest::{
    self,
    header::{AUTHORIZATION, USER_AGENT},
};

const GITHUB_GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn values(&self, _ctx: &Context<'_>) -> Result<impl Stream<Item = i32>> {
        Ok(futures_util::stream::once(async move { 10 }))
    }
}

pub struct Package(pub LibPackage);

#[Object]
impl Package {
    async fn name(&self) -> &str {
        &self.0.name
    }
}

#[derive(SimpleObject)]
pub struct PublisherOutput {
    pub name: String,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn publishers(&self, ctx: &Context<'_>) -> Result<Vec<PublisherOutput>> {
        let db = ctx.data_unchecked::<AsyncDatabase>();
        let list = Publisher::all_async(db).await?;
        Ok(list
            .into_iter()
            .map(|ns| PublisherOutput {
                name: ns.contents.name,
            })
            .collect())
    }

    async fn packages(&self, _ctx: &Context<'_>) -> Result<Vec<Package>> {
        Ok(vec![Package(
            PackageBuilder::default()
                .name(String::from("test"))
                .build()?,
        )])
    }
}

#[derive(InputObject)]
pub struct PackageInput {
    name: String,
    publisher: String,
    document: String,
}

#[derive(InputObject)]
pub struct TokenInput {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[derive(SimpleObject)]
pub struct RegisterOutput {
    username: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../github_schema.graphql",
    query_path = "../github_username_query.graphql",
    response_derives = "Debug"
)]
struct GitHubUsernameQuery;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_package(&self, ctx: &Context<'_>, pkg: PackageInput) -> Result<Package> {
        let decoded = base64::engine::general_purpose::STANDARD.decode(&pkg.document)?;
        let document_string = String::from_utf8(decoded)?;
        let decoded_document = serde_json::from_str(&document_string)?;
        let db_pkg = database::Package {
            name: pkg.name,
            publisher: pkg.publisher,
            document: decoded_document,
            manifests: vec![],
        };

        let storage = ctx.data_unchecked::<AsyncStorage>();
        let db = storage
            .database::<database::ForgeSchema>(database::DATABASE_NAME)
            .await?;
        let res = db_pkg.push_into_async(&db).await?;
        Ok(Package(res.contents.document))
    }

    async fn create_publisher(
        &self,
        ctx: &Context<'_>,
        name: String,
        owners: Vec<String>,
    ) -> Result<PublisherOutput> {
        let publisher = Publisher {
            name,
            public: true,
            owners, //TODO Two owners or must be admin which is checked by if user has role
        };
        let storage = ctx.data_unchecked::<AsyncStorage>();
        let db = storage
            .database::<database::ForgeSchema>(database::DATABASE_NAME)
            .await?;
        let res = publisher.push_into_async(&db).await?;

        Ok(PublisherOutput {
            name: res.contents.name,
        })
    }

    async fn register(&self, ctx: &Context<'_>, token: TokenInput) -> Result<RegisterOutput> {
        let request_body = GitHubUsernameQuery::build_query(git_hub_username_query::Variables);
        let auth_header = format!("Bearer {}", &token.access_token);
        let client = reqwest::Client::new();
        let resp = client
            .post(GITHUB_GRAPHQL_API_URL)
            .header(USER_AGENT, "Package Forge V1.0")
            .header(AUTHORIZATION, auth_header)
            .json(&request_body)
            .send()
            .await?;
        let response_body: Response<git_hub_username_query::ResponseData> = resp.json().await?;
        let username = if let Some(error) = response_body.errors {
            Err(format!(
                "GitHub API Error: {}",
                error
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
            ))
        } else if let Some(data) = response_body.data {
            Ok(data.viewer.login)
        } else {
            Err(format!("GitHub API Error: Response empty"))
        }?;

        let storage = ctx.data_unchecked::<AsyncStorage>();
        let mut tx = Transaction::new();
        //let user_id = storage.create_user(&username).await?;
        //let admin = storage.admin().await;
        //TODO Permissions
        let profile = Profile {
            username: username.clone(),
            token: Some(crate::database::GitHubToken {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_in: token.expires_in,
            }),
            ssh_pub_keys: vec![],
            gpg_pub_keys: vec![],
        };
        tx.push(Operation::push_serialized::<Profile>(&profile)?);
        let publisher = Publisher {
            name: format!("~{}", &username),
            owners: vec![username.clone()],
            public: false,
        };
        tx.push(Operation::push_serialized::<Publisher>(&publisher)?);
        let db = storage
            .database::<database::ForgeSchema>(database::DATABASE_NAME)
            .await?;
        tx.apply_async(&db).await?;

        //let home_permission_group = PermissionGroup {
        //    name: format!("home-publisher-{}", &username),
        //    statements: vec![Statement::for_resource(ResourceName::named("profiles"))
        //        .allowing_all()
        //        .with("username", Configuration::String(username.clone()))],
        //};

        Ok(RegisterOutput {
            username: username.clone(),
        })
    }
}
