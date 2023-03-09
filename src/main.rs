use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};
use dotenvy::dotenv;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use std::env;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

use starter_rust_rest_api::configuration::AppState;
use starter_rust_rest_api::routes::*;

#[tokio::main]
async fn main() {
    // load environment variables
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect(".env must have DATABASE_URL");

    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // initialize database
    let db: DatabaseConnection = Database::connect(&database_url)
        .await
        .expect("Database connection failed");
    Migrator::up(&db, None).await.unwrap();

    // initialize session layer
    let store = MemoryStore::new();
    let secret = b"super-long-and-secret-random-key-needed-to-verify-message-integrity";
    let session_layer = SessionLayer::new(store, secret).with_secure(false);

    // initialize app state
    let state = AppState { db };

    // build the app
    let app = Router::new()
        .route("/", get(display_handler))
        .route("/inc", get(increment_handler))
        .route("/reset", get(reset_handler))
        .route("/posts", get(get_posts).post(create_post))
        .route(
            "/posts/:post_id",
            get(get_post).put(update_post).delete(delete_post),
        )
        .route(
            "/users",
            get(get_current_user)
                .post(create_user)
                .put(update_current_user),
        )
        .route("/users/:user_id", get(get_user).delete(delete_user))
        .route("/signin", post(signin))
        .route("/signout", post(signout))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // run the app with hyper on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// # Session routes
async fn display_handler(session: ReadableSession) -> impl IntoResponse {
    let mut count = 0;
    count = session.get("count").unwrap_or(count);
    format!(
        "Count is: {}; visit /inc to increment and /reset to reset",
        count
    )
}

async fn increment_handler(mut session: WritableSession) -> impl IntoResponse {
    let mut count = 1;
    count = session.get("count").map(|n: i32| n + 1).unwrap_or(count);
    session.insert("count", count).unwrap();
    format!("Count is: {}", count)
}

async fn reset_handler(mut session: WritableSession) -> impl IntoResponse {
    session.destroy();
    "Count reset"
}
