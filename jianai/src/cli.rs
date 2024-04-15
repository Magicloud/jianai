use clap::*;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::PathBuf;

#[derive(Parser, Clone, Debug)]
pub struct PGParams {
    #[arg(long, env = "PGHOST")]
    pub pg_host: String,
    #[arg(long, env = "PGPORT", default_value = "5432")]
    pub pg_port: u16,
    #[arg(long, env = "PGDATABASE")]
    pub pg_database: String,
    #[arg(long, env = "PGUSERNAME")]
    pub pg_username: String,
    #[arg(long, env = "PGPASSWORD")]
    pub pg_password: String,
}
impl PGParams {
    pub fn get_conn_str(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.pg_username,
            utf8_percent_encode(&self.pg_password, NON_ALPHANUMERIC),
            self.pg_host,
            self.pg_port,
            self.pg_database
        )
    }
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Params {
    #[command(flatten)]
    pub pg_params: PGParams,
    #[arg(short, long, default_value = "redis://localhost:6379/")]
    pub redis_address: String,
    #[arg(short, long)]
    pub image_folder: PathBuf,
    #[arg(short, long, default_value = "localhost:3000")]
    pub listen_address: String,

    #[command(subcommand)]
    pub cmd: SubCmd,
}

#[derive(Subcommand, Clone, Debug)]
#[command(rename_all = "lower")]
pub enum SubCmd {
    Store {
        #[arg(short, long, default_value = "/upload_image")]
        upload_image_http_path: String,
    },
    Segment {
        #[arg(short = 'p', long)]
        model_path: PathBuf,
    },
}
