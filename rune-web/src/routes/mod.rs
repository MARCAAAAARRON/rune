pub mod commits;
pub mod tree;
pub mod blob;
pub mod diff;
pub mod refs;
use axum::response::Redirect;

pub async fn index() -> Redirect {
    Redirect::temporary("/commits")
}
