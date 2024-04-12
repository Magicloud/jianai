mod handlers;

use rocket::routes;

#[tokio::main]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let web = tokio::spawn(
        rocket::build()
            .mount("/healthz", routes![handlers::healthz])
            .mount("/status", routes![handlers::status])
            .launch(),
    );
    web.await??;
    Ok(())
}
