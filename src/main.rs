use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum_sessions::{
    async_session::MemoryStore,
    extractors::{ReadableSession, WritableSession},
    SessionLayer,
};
use dotenvy::dotenv;
use entity::post;
use entity::post::Entity as Post;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection, EntityTrait, ModelTrait,
};
use serde_json::Value;
use std::env;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

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
        .route(
            "/posts",
            get(get_posts).post(create_post).delete(delete_post),
        )
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

// # Database routes
async fn get_posts(State(state): State<AppState>) -> Json<Vec<Value>> {
    let posts = Post::find()
        .into_json()
        .all(&state.db)
        .await
        .expect("Cannot retrieve posts");
    
    Json(posts)
}

async fn create_post(State(state): State<AppState>) -> impl IntoResponse {
    let new_post = post::ActiveModel {
        title: Set("Post title".to_owned()),
        text: Set("Post description".to_owned()),
        ..Default::default() // all other attributes are `NotSet`
    };

    let new_post = new_post
        .insert(&state.db)
        .await
        .expect("Cannot create post");

    Json(new_post)
}

#[derive(serde::Deserialize)]
struct DeleteQuery {
    post_id: i32,
}
async fn delete_post(
    State(state): State<AppState>,
    query: Query<DeleteQuery>,
) -> impl IntoResponse {
    let post = Post::find_by_id(query.0.post_id)
        .one(&state.db)
        .await
        .expect("Cannot access database");
    
    match post {
        None => (StatusCode::NOT_FOUND, format!("Post not found")),
        Some(post) => {
            let res = post.delete(&state.db).await.expect("Cannot delete post");
            (
                StatusCode::OK,
                format!("Deleted {} posts", res.rows_affected),
            )
        }
    }
}
