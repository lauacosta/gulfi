use argon2::{Argon2, PasswordHasher};
use camino::Utf8Path;
use color_eyre::owo_colors::OwoColorize;
use gulfi_sqlite::get_vec_conn;
use password_hash::{SaltString, rand_core::OsRng};
use rusqlite::params;

use crate::CliError;

pub fn create_user<P: AsRef<Utf8Path>>(
    db_path: P,
    username: &str,
    password: &str,
) -> Result<(), CliError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    let conn = get_vec_conn(db_path)?;
    let updated = conn.execute(
        "insert or replace into users(username, password_hash) values (?,?)",
        params![username, password_hash],
    )?;

    assert_eq!(updated, 1);

    println!("User {} was created", username.bold().bright_green());
    Ok(())
}
