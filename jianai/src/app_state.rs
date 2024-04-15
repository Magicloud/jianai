use derivative::Derivative;
use diesel_async::{pooled_connection::bb8::Pool, AsyncPgConnection};
use redis::{aio::MultiplexedConnection, Client};
use redis_pool::RedisPool;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct StoreState {
    #[derivative(Debug = "ignore")]
    pub redis_pool: RedisPool<Client, MultiplexedConnection>,
    pub pg_pool: Pool<AsyncPgConnection>,
    pub id_prefix: String,
    pub id_counter: AtomicU64,
    pub upload_image_http_path: String,
    pub image_folder: PathBuf,
}
impl StoreState {
    pub fn get_id(&self) -> String {
        format!(
            "{}{}",
            self.id_prefix,
            self.id_counter.fetch_add(1, Ordering::SeqCst)
        )
    }
}
