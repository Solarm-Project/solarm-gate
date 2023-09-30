mod api;
mod database;

use crate::api::MutationRoot;
use crate::api::QueryRoot;
use crate::api::SubscriptionRoot;
use async_graphql::{
    http::{GraphiQLSource, ALL_WEBSOCKET_PROTOCOLS},
    Data, Schema,
};
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{Extension, WebSocketUpgrade},
    http::HeaderMap,
    response::{self, IntoResponse, Response},
    routing::get,
    Router,
};
use bonsaidb::local::{
    config::{Builder, StorageConfiguration},
    AsyncDatabase, AsyncStorage,
};
use clap::Parser;
use miette::IntoDiagnostic;
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};

type ForgeSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct DatabaseDataRefs {
    pub namespaces: AsyncDatabase,
}

async fn graphql_handler(
    schema: Extension<ForgeSchema>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut req = req.into_inner();
    if let Some(token) = get_token_from_headers(&headers) {
        req = req.data(token);
    }
    schema.execute(req).await.into()
}

fn get_token_from_headers(headers: &HeaderMap) -> Option<Token> {
    headers
        .get("Token")
        .and_then(|value| value.to_str().map(|s| Token(s.to_string())).ok())
}

async fn graphiql() -> impl IntoResponse {
    response::Html(
        GraphiQLSource::build()
            .endpoint("/")
            .subscription_endpoint("/ws")
            .finish(),
    )
}

async fn graphql_ws_handler(
    Extension(schema): Extension<ForgeSchema>,
    protocol: GraphQLProtocol,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, schema.clone(), protocol)
                .on_connection_init(on_connection_init)
                .serve()
        })
}

pub struct Token(pub String);

// For more details see:
// https://github.com/enisdenjo/graphql-ws/blob/master/PROTOCOL.md#connectioninit
pub async fn on_connection_init(value: serde_json::Value) -> async_graphql::Result<Data> {
    #[derive(Deserialize)]
    struct Payload {
        token: String,
    }

    // Coerce the connection params into our `Payload` struct so we can
    // validate the token exists in the headers.
    if let Ok(payload) = serde_json::from_value::<Payload>(value) {
        let mut data = Data::default();
        data.insert(Token(payload.token));
        Ok(data)
    } else {
        Err("Token is required".into())
    }
}

#[derive(Parser, Debug)]
struct CLIArgs {
    #[arg(long, short, default_value = "forge.bonsaidb")]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let args = CLIArgs::parse();

    let db_config = StorageConfiguration::new(args.data_dir)
        .with_schema::<database::ForgeSchema>()
        .into_diagnostic()?;

    let storage = AsyncStorage::open(db_config).await.into_diagnostic()?;

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(storage)
        .finish();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(graphiql).post(graphql_handler))
        .route("/ws", get(graphql_ws_handler))
        .layer(Extension(schema));

    println!("GraphiQL IDE: http://localhost:3000");

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .into_diagnostic()?;

    Ok(())
}
