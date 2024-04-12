use rocket::{http::Status, *};

#[get("/healthz")]
pub fn healthz() -> (Status, ()) {
    (Status::Ok, ())
}

#[get("/status")]
pub fn status() -> (Status, String) {
    (Status::Ok, String::new())
}
