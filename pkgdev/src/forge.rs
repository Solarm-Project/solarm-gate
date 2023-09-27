use crate::config::GitHubToken;
use clap::Subcommand;
use graphql_client::{GraphQLQuery, Response};
use miette::{IntoDiagnostic, Result};
use oauth2::basic::BasicClient;
use oauth2::devicecode::StandardDeviceAuthorizationResponse;
use oauth2::reqwest::http_client;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, DeviceAuthorizationUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use std::env;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../github_schema.graphql",
    query_path = "../github_username_query.graphql",
    response_derives = "Debug"
)]
pub struct GitHubUsernameQuery;

#[derive(Subcommand, Debug, Clone)]
pub enum ForgeCLI {
    Login,
    Username,
}

pub fn handle_forge(cmd: &ForgeCLI) -> Result<()> {
    match cmd {
        ForgeCLI::Login => {
            dotenv::dotenv().into_diagnostic()?;
            let token = get_device_token()?;
            let mut config = crate::config::Settings::open()?;
            config.github_token = Some(token);
            config.save().into_diagnostic()
        }
        ForgeCLI::Username => {
            let config = crate::config::Settings::open()?;

            if let Some(github_token) = config.github_token {
                let request_body =
                    GitHubUsernameQuery::build_query(git_hub_username_query::Variables);
                let auth_header = format!("Bearer {}", github_token.access_token);
                let client = reqwest::blocking::Client::new();
                let resp = client
                    .post("https://api.github.com/graphql")
                    .header(USER_AGENT, "Ports Cli")
                    .header(AUTHORIZATION, auth_header)
                    .json(&request_body)
                    .send()
                    .into_diagnostic()?;
                let response_body: Response<git_hub_username_query::ResponseData> =
                    resp.json().into_diagnostic()?;
                if let Some(data) = response_body.data {
                    println!("The logged in GitHub Account is: {}", data.viewer.login);
                } else {
                    println!("Not logged into GitHub");
                }
            }

            Ok(())
        }
    }
}

const GITHUB_DEVICE_AUTH_URL: &str = "https://github.com/login/device/code";
const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub fn get_device_token() -> miette::Result<GitHubToken> {
    let device_auth_url =
        DeviceAuthorizationUrl::new(GITHUB_DEVICE_AUTH_URL.to_string()).into_diagnostic()?;
    let github_client_id = ClientId::new(
        env::var("GITHUB_CLIENT_ID").expect("Missing the GITHUB_CLIENT_ID environment variable."),
    );
    let github_client_secret = ClientSecret::new(
        env::var("GITHUB_CLIENT_SECRET")
            .expect("Missing the GITHUB_CLIENT_SECRET environment variable."),
    );
    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        AuthUrl::new(GITHUB_AUTH_URL.to_string()).into_diagnostic()?,
        Some(TokenUrl::new(GITHUB_TOKEN_URL.to_string()).into_diagnostic()?),
    )
    .set_device_authorization_url(device_auth_url);

    let details: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .into_diagnostic()?
        .add_scope(Scope::new("public_repo".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .add_scope(Scope::new("read:gpg_key".to_string()))
        .add_scope(Scope::new("read:org".to_string()))
        .request(http_client)
        .into_diagnostic()?;

    println!(
        "Open this URL in your browser:\n{}\nand enter the code: {}\npress Enter once done",
        details.verification_uri().to_string(),
        details.user_code().secret().to_string()
    );

    let term = console::Term::stdout();

    term.read_key().into_diagnostic()?;

    let token_result = client
        .exchange_device_access_token(&details)
        .request(http_client, std::thread::sleep, None)
        .into_diagnostic()?;

    let gh_token = GitHubToken {
        access_token: token_result.access_token().secret().clone(),
        refresh_token: token_result.refresh_token().map(|o| o.secret().clone()),
        token_type: String::from("Bearer"),
        expires_in: token_result.expires_in().map(|o| o.as_secs()),
        scope: token_result
            .scopes()
            .map(|o| o.into_iter().map(|s| s.to_string()).collect()),
    };

    Ok(gh_token)
}
