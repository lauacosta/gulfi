use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State};
use rand::Rng;
use rand::distr::Alphanumeric;
use rusqlite::params;
use serde::Deserialize;
use serde_json::json;
use tracing::{info_span, warn};

use crate::{into_http::HttpError, startup::ServerState};

#[derive(Deserialize, Debug)]
pub(crate) struct AuthParams {
    username: String,
    password: String,
}

#[tracing::instrument(skip(app, payload), name = "generando nuevo auth_token", fields(username =  %payload.username))]
pub async fn auth(
    State(app): State<ServerState>,
    Json(payload): Json<AuthParams>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let conn = app.pool.acquire().await?;

    let username = payload.username;
    let password = payload.password;

    let fetch_span = info_span!("auth.fetch_user", %username);
    let _guard = fetch_span.enter();

    let mut stmt =
        conn.prepare_cached("SELECT id, username, password_hash FROM users WHERE username = ?")?;

    let (id, username, password_hash) = stmt.query_row(params![username], |row| {
        let id: usize = row.get(0)?;
        let username: String = row.get(1)?;
        let password_hash: String = row.get(2)?;

        Ok((id, username, password_hash))
    })?;

    let parsed_hash = match PasswordHash::new(&password_hash) {
        Ok(h) => h,
        Err(err) => {
            warn!("invalid password_hash!: {password_hash}");
            return Err(HttpError::from(err));
        }
    };

    {
        let verify_span = info_span!("auth.verify_password", %username);
        let _guard = verify_span.enter();

        let argon2 = Argon2::default();
        if let Err(err) = argon2.verify_password(password.as_bytes(), &parsed_hash) {
            warn!(%username, error = ?err, "Password verification failed");
            return Err(HttpError::from(err));
        }
    }

    let token: String = {
        let generate_span = info_span!("auth.generate_token_string", %username);
        let _guard = generate_span.enter();
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    };

    let mut stmt = conn.prepare_cached("UPDATE users SET auth_token = ? WHERE username = ?")?;
    stmt.execute(params![token, username])?;

    Ok(Json(
        json!({"id": id, "username": username, "auth_token": token}),
    ))
}
