use std::path::Path;

use argon2::{Argon2, PasswordHasher};
use color_eyre::owo_colors::OwoColorize;
use gulfi_sqlite::spawn_vec_connection;
use password_hash::{SaltString, rand_core::OsRng};
use rusqlite::params;

use crate::CliError;

pub fn create_user<P: AsRef<Path>>(
    db_path: P,
    username: &str,
    password: &str,
) -> Result<(), CliError> {
    let conn = spawn_vec_connection(db_path)?;

    conn.execute_batch(
        "create table if not exists users (
            id integer primary key autoincrement,
            username text not null unique,
            password_hash text not null,
            auth_token text,
            created_at datetime default current_timestamp,
            updated_at datetime default current_timestamp
        )",
    )?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    let updated = conn.execute(
        "insert or replace into users(username, password_hash) values (?,?)",
        params![username, password_hash],
    )?;

    assert_eq!(updated, 1);

    println!("User {} was created", username.bold().bright_green());
    Ok(())
}
