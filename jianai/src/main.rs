#![feature(try_blocks)]
mod app_state;
mod cli;
mod handlers;
mod model;
mod schema;
mod types;

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::Params;
use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use handlers::*;
use redis_pool::RedisPool;
use rocket::routes;
use std::sync::atomic::AtomicU64;
use tracing_subscriber::prelude::*;

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "storing=info,tower_http=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Params::parse();

    let redis_client = redis::Client::open(args.redis_address)?;
    let redis_pool = RedisPool::from(redis_client);

    let pg_manager =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(args.pg_params.get_conn_str());
    let pg_pool = Pool::builder().build(pg_manager).await?;

    let mut web = rocket::build()
        .mount("/healthz", routes![healthz])
        .mount("/status", routes![status]);

    match args.cmd {
        cli::SubCmd::Store {
            upload_image_http_path,
        } => {
            let state = app_state::AppState {
                redis_pool,
                pg_pool,
                id_prefix: format!(
                    "{}{}",
                    gethostname::gethostname()
                        .into_string()
                        .map_err(|e| { anyhow!("{e:?}") })?,
                    std::process::id()
                ),
                id_counter: AtomicU64::new(0),
                upload_image_http_path: upload_image_http_path,
                image_folder: args.image_folder,
            };
            web = web
                .mount("/upload_meta", routes![upload_meta])
                .mount("/upload_image", routes![upload_image])
                .manage(state);
            web.launch().await?;
        }
    }

    // let web = tokio::spawn(
    // web.await??;
    Ok(())
}
