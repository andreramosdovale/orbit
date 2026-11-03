use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7700").await.unwrap();
    println!("sun ouvindo em {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
