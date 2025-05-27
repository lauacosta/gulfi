use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State};
use rand::Rng;
use rand::distr::Alphanumeric;
use rusqlite::params;
use serde::Deserialize;
use serde_json::json;

use crate::{into_http::HttpError, startup::ServerState};

#[derive(Deserialize, Debug)]
pub(crate) struct AuthParams {
    username: String,
    password: String,
}

#[tracing::instrument(skip(app, payload), name = "generando nuevo auth_token")]
pub async fn auth(
    State(app): State<ServerState>,
    Json(payload): Json<AuthParams>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let conn_handle = app.pool.acquire().await?;

    let mut stmt =
        conn_handle.prepare("SELECT id, username, password_hash FROM users WHERE username = ?")?;

    let username = payload.username;
    let password = payload.password;

    let (id, username, password_hash) = stmt.query_row(params![username], |row| {
        let id: usize = row.get(0)?;
        let username: String = row.get(1)?;
        let password_hash: String = row.get(2)?;

        Ok((id, username, password_hash))
    })?;

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|err| eyre::eyre!("Invalid password hash: {err}"))
        .expect("TODO");

    let argon2 = Argon2::default();

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .expect("TODO");

    let token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    conn_handle.execute(
        "UPDATE users SET auth_token = ? WHERE username = ?",
        params![token, username],
    )?;

    Ok(Json(
        json!({"id": id, "username": username, "auth_token": token}),
    ))
}
