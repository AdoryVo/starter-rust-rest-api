use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use entity::post;
use entity::post::Entity as Post;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, ModelTrait};
use serde::Deserialize;
use serde_json::Value;

use crate::configuration::AppState;

#[derive(Deserialize)]
pub struct IdPath {
    post_id: i32,
}

#[derive(Deserialize)]
pub struct PostForm {
    title: String,
    text: String,
}

// POST /posts
pub async fn create_post(
    State(state): State<AppState>,
    Json(payload): Json<PostForm>,
) -> impl IntoResponse {
    let new_post = post::ActiveModel {
        title: Set(payload.title.to_owned()),
        text: Set(payload.text.to_owned()),
        ..Default::default() // all other attributes are `NotSet`
    };

    let new_post = new_post
        .insert(&state.db)
        .await
        .expect("Cannot create post");

    (StatusCode::CREATED, Json(new_post))
}

// GET /posts
pub async fn get_posts(State(state): State<AppState>) -> Json<Vec<Value>> {
    let posts = Post::find()
        .into_json()
        .all(&state.db)
        .await
        .expect("Cannot retrieve posts");

    Json(posts)
}

// GET /posts/:post_id
pub async fn get_post(
    State(state): State<AppState>,
    Path(IdPath { post_id }): Path<IdPath>,
) -> impl IntoResponse {
    let post = Post::find_by_id(post_id)
        .one(&state.db)
        .await
        .expect("Cannot retrieve posts");

    match post {
        None => (StatusCode::NOT_FOUND, Json(post)),
        Some(_) => (StatusCode::OK, Json(post)),
    }
}

// PUT /posts/:post_id
pub async fn update_post(
    State(state): State<AppState>,
    Path(IdPath { post_id }): Path<IdPath>,
    Json(update): Json<PostForm>,
) -> impl IntoResponse {
    let post_result = Post::find_by_id(post_id)
        .one(&state.db)
        .await
        .expect("Cannot retrieve posts");

    match post_result {
        None => StatusCode::NOT_FOUND,
        Some(post) => {
            let mut post: post::ActiveModel = post.into();
            post.title = Set(update.title.to_owned());
            post.text = Set(update.text.to_owned());
            post.update(&state.db).await.expect("Cannot delete post");

            StatusCode::NO_CONTENT
        }
    }
}

// DELETE /posts/:post_id
pub async fn delete_post(
    State(state): State<AppState>,
    Path(IdPath { post_id }): Path<IdPath>,
) -> impl IntoResponse {
    let post = Post::find_by_id(post_id)
        .one(&state.db)
        .await
        .expect("Cannot access database");

    match post {
        None => StatusCode::NOT_FOUND,
        Some(post) => {
            post.delete(&state.db).await.expect("Cannot delete post");
            StatusCode::NO_CONTENT
        }
    }
}
