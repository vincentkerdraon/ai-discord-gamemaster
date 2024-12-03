use axum::{response::Html, routing::get, Router};
use tracing::info;

async fn hello_world() -> Html<&'static str> {
    Html("Hello, World!")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting server...");

    // build our application with a route
    let app = Router::new().route("/", get(hello_world));

    // run it
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service()) // Convert the router into a make service
        .await
        .unwrap();
}
