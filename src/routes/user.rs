use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_sessions::extractors::{ReadableSession, WritableSession};
use entity::user;
use entity::user::Entity as User;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, ModelTrait, QueryFilter,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::configuration::AppState;

#[derive(Deserialize)]
pub struct IdPath {
    user_id: Uuid,
}

#[derive(Deserialize)]
pub struct UserForm {
    email: String,
    password: String,
}

fn hash_password(password: String) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let result = argon2.hash_password(password.as_bytes(), &salt)?;

    Ok(result.to_string())
}

// POST /users
pub async fn create_user(
    mut session: WritableSession,
    State(state): State<AppState>,
    Json(payload): Json<UserForm>,
) -> impl IntoResponse {
    if let Ok(password_hash) = hash_password(payload.password) {
        let new_user = user::ActiveModel {
            email: Set(payload.email.to_owned()),
            password_hash: Set(password_hash.to_owned()),
            ..Default::default() // all other attributes are `NotSet`
        };

        let result = new_user.insert(&state.db).await;

        match result {
            Ok(new_user) => {
                session.insert("id", new_user.id).unwrap();
                (StatusCode::CREATED, Json(Some(new_user)))
            }
            Err(error) => match error {
                sea_orm::DbErr::Query(_) => (StatusCode::CONFLICT, Json(None)),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
            },
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
    }
}

// POST /signin
pub async fn signin(
    mut session: WritableSession,
    State(state): State<AppState>,
    Json(payload): Json<UserForm>,
) -> impl IntoResponse {
    let user = User::find()
        .filter(user::Column::Email.eq(payload.email))
        .one(&state.db)
        .await
        .expect("Cannot retrieve user");

    match user {
        None => StatusCode::UNAUTHORIZED,
        Some(user) => {
            let parsed_hash = PasswordHash::new(&user.password_hash).expect("Cannot hash password");

            if Argon2::default()
                .verify_password(payload.password.as_bytes(), &parsed_hash)
                .is_ok()
            {
                session.insert("id", user.id).unwrap();
                StatusCode::NO_CONTENT
            } else {
                StatusCode::UNAUTHORIZED
            }
        }
    }
}

// POST /signout
pub async fn signout(mut session: WritableSession) -> impl IntoResponse {
    session.destroy();

    StatusCode::NO_CONTENT
}

// GET /users
pub async fn get_current_user(
    session: ReadableSession,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(user_id) = session.get::<Uuid>("id") {
        let user = User::find_by_id(user_id)
            .one(&state.db)
            .await
            .expect("Cannot retrieve users");

        match user {
            None => (StatusCode::NOT_FOUND, Json(user)),
            Some(_) => (StatusCode::OK, Json(user)),
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(None))
    }
}

// unused
pub async fn get_users(State(state): State<AppState>) -> Json<Vec<Value>> {
    let users = User::find()
        .into_json()
        .all(&state.db)
        .await
        .expect("Cannot retrieve users");

    Json(users)
}

// GET /users/:user_id
pub async fn get_user(
    State(state): State<AppState>,
    Path(IdPath { user_id }): Path<IdPath>,
) -> impl IntoResponse {
    let user = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .expect("Cannot retrieve users");

    match user {
        None => (StatusCode::NOT_FOUND, Json(user)),
        Some(_) => (StatusCode::OK, Json(user)),
    }
}

// PUT /users
pub async fn update_current_user(
    session: ReadableSession,
    State(state): State<AppState>,
    Json(update): Json<UserForm>,
) -> impl IntoResponse {
    if let Some(user_id) = session.get::<Uuid>("id") {
        let user_result = User::find_by_id(user_id)
            .one(&state.db)
            .await
            .expect("Cannot retrieve users");

        match user_result {
            None => StatusCode::NOT_FOUND,
            Some(user) => {
                let mut user: user::ActiveModel = user.into();
                user.email = Set(update.email.to_owned());
                if update.password != "" {
                    user.password_hash = Set(update.password.to_owned());
                }
                user.update(&state.db).await.expect("Cannot delete user");

                StatusCode::NO_CONTENT
            }
        }
    } else {
        StatusCode::UNAUTHORIZED
    }
}

// PUT /users/:user_id
pub async fn update_user(
    State(state): State<AppState>,
    Path(IdPath { user_id }): Path<IdPath>,
    Json(update): Json<UserForm>,
) -> impl IntoResponse {
    let user_result = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .expect("Cannot retrieve users");

    match user_result {
        None => StatusCode::NOT_FOUND,
        Some(user) => {
            let mut user: user::ActiveModel = user.into();
            user.email = Set(update.email.to_owned());
            user.password_hash = Set(update.password.to_owned());
            user.update(&state.db).await.expect("Cannot delete user");

            StatusCode::NO_CONTENT
        }
    }
}

// DELETE /users/:user_id
pub async fn delete_user(
    State(state): State<AppState>,
    Path(IdPath { user_id }): Path<IdPath>,
) -> impl IntoResponse {
    let user = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .expect("Cannot access database");

    match user {
        None => StatusCode::NOT_FOUND,
        Some(user) => {
            user.delete(&state.db).await.expect("Cannot delete user");
            StatusCode::NO_CONTENT
        }
    }
}
