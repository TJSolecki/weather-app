use axum::routing::get;
use axum::Router;
#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/hello", get(hello_world));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}", "127.0.0.1:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn hello_world() -> &'static str {
    "Hello world!"
}
