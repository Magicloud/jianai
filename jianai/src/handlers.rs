use crate::app_state::AppState;
use crate::schema::*;
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use redis::AsyncCommands;
use rocket::{
    fs::TempFile,
    http::{Header, Status},
    *,
};
use sha3::{Digest, Sha3_224};
use tracing::instrument;

#[get("/healthz")]
pub fn healthz() -> (Status, ()) {
    (Status::Ok, ())
}

#[get("/status")]
pub fn status() -> (Status, String) {
    (Status::Ok, String::new())
}

#[derive(Responder)]
struct UploadMetaResponse {
    inner: (Status, String), // this is so weird
    location_header: Header<'static>,
}

#[instrument]
#[post("/upload_meta", format = "application/json", data = "<meta>")]
pub async fn upload_meta(state: &State<AppState>, meta: String) -> UploadMetaResponse {
    let upload_id = state.get_id();
    let result: anyhow::Result<()> = try {
        let mut redis = state.redis_pool.aquire().await?;
        redis.set(&upload_id, meta).await?;
    };
    match result {
        Ok(_) => {
            info!("Ready to receive");
            UploadMetaResponse {
                inner: (Status::Ok, String::new()),
                location_header: Header::new(
                    "LOCATION",
                    format!("{}?upload_id={upload_id}", state.upload_image_http_path),
                ),
            }
        }
        Err(e) => UploadMetaResponse {
            inner: (Status::InternalServerError, format!("{e:?}")),
            location_header: Header::new("", ""),
        },
    }
}

#[instrument]
#[post("/upload_image?<upload_id>", format = "plain", data = "<file>")]
pub async fn upload_image(
    state: &State<AppState>,
    upload_id: String,
    mut file: TempFile<'_>,
) -> (Status, String) {
    let result: anyhow::Result<()> = try {
        let mut redis = state.redis_pool.aquire().await?;
        let meta: String = redis.get(&upload_id).await?;
        let meta: serde_json::Value = serde_json::from_str(&meta)?;

        let filename = state.image_folder.clone().join(&upload_id);
        file.persist_to(&filename).await?;

        let mut hasher = Sha3_224::new();
        let content = tokio::fs::read(filename).await?;
        hasher.update(&content);
        let digest = hasher.finalize();

        let mut pg_conn = state.pg_pool.get().await?;
        diesel::insert_into(images::table)
            .values((
                images::filename.eq(&upload_id),
                images::digest.eq(digest.as_slice()),
                images::metadata.eq(&meta),
            ))
            .execute(&mut pg_conn)
            .await?;

        redis.del::<String, String>(upload_id).await?;
    };
    match result {
        Ok(_) => (Status::Ok, String::new()),
        Err(e) => (Status::InternalServerError, format!("{e:?}")),
    }
}
