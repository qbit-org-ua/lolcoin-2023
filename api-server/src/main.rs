use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

mod errors;
mod integers;
mod modules;

pub(crate) const LOGGER_MSG: &str = "lolcoin_api";

pub(crate) type Result<T> = std::result::Result<T, crate::errors::Error>;

fn get_cors() -> Cors {
    Cors::permissive()
        .allowed_methods(vec!["GET"])
        .allowed_headers(vec![
            actix_web::http::header::AUTHORIZATION,
            actix_web::http::header::ACCEPT,
        ])
        .allowed_header(actix_web::http::header::CONTENT_TYPE)
        .max_age(3600)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let env_filter =
        tracing_subscriber::EnvFilter::new("near=info,near_jsonrpc_client=warn,lolcoin_api=debug");
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    tracing::debug!(
        target: crate::LOGGER_MSG,
        "NEAR Enhanced API Server is initializing..."
    );

    let rpc_url = &std::env::var("RPC_URL").expect("failed to get RPC url");
    let rpc_client = near_jsonrpc_client::JsonRpcClient::connect(rpc_url);

    let server = HttpServer::new(move || {
        let json_config = web::JsonConfig::default();

        let mut app = App::new()
            .app_data(json_config)
            .wrap(actix_web::middleware::Logger::default())
            .app_data(web::Data::new(rpc_client.clone()))
            .wrap(get_cors());

        app = app.configure(self::modules::coins::register_services);

        app
    })
    .bind("0.0.0.0:9001")
    .unwrap()
    .shutdown_timeout(5)
    .run();

    tracing::debug!(
        target: crate::LOGGER_MSG,
        "LOLcoin API server is starting..."
    );

    server.await
}
