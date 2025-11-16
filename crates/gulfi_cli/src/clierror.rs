use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Could not parse metadata file: {0}")]
    MetaParseError(#[from] serde_json::Error),
    #[error("Failed to open metadata file: {0}")]
    MetaOpenError(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("Config error: {0}")]
    ConfigError(#[from] config::ConfigError),
    #[error("Other: {0}")]
    Other(#[from] eyre::Report),
    #[error("Password hashing failed: {0}")]
    HashingError(password_hash::Error),
}

impl CliError {
    pub fn print_tip(&self) {
        match self {
            CliError::MetaParseError(error) => {
                eprintln!("💡 Failed to parse metadata file:");
                match error {
                    serde_json::Error { .. } if error.is_syntax() => {
                        eprintln!(
                            "   • JSON syntax error at line {}, column {}",
                            error.line(),
                            error.column()
                        );
                        eprintln!(
                            "   • Check for missing commas, brackets, or quotes near this location"
                        );
                        eprintln!("   • Look for trailing commas or unclosed strings");
                    }
                    serde_json::Error { .. } if error.is_data() => {
                        eprintln!("   • Data format error: {error}");
                        eprintln!("   • JSON structure doesn't match expected format");
                    }
                    serde_json::Error { .. } if error.is_eof() => {
                        eprintln!("   • Unexpected end of file");
                        eprintln!("   • metadata file appears to be incomplete");
                    }
                    serde_json::Error { .. } if error.is_io() => {
                        eprintln!("   • I/O error while reading JSON: {error}");
                    }
                    _ => {
                        eprintln!("   • JSON parsing error: {error}");
                    }
                }
            }
            CliError::MetaOpenError(error) => {
                eprintln!("💡 Cannot open metadata file file:");
                match error.kind() {
                    std::io::ErrorKind::NotFound => {
                        eprintln!("   • metadata file file not found");
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        eprintln!("   • Permission denied - check file permissions");
                    }
                    std::io::ErrorKind::AlreadyExists => {
                        eprintln!("   • File already exists (unexpected in this context)");
                    }
                    std::io::ErrorKind::WouldBlock => {
                        eprintln!("   • File operation would block");
                    }
                    std::io::ErrorKind::InvalidInput => {
                        eprintln!("   • Invalid file path or name");
                    }
                    std::io::ErrorKind::InvalidData => {
                        eprintln!("   • File contains invalid data");
                    }
                    std::io::ErrorKind::TimedOut => {
                        eprintln!("   • File operation timed out");
                    }
                    std::io::ErrorKind::Interrupted => {
                        eprintln!("   • File operation was interrupted");
                    }
                    std::io::ErrorKind::OutOfMemory => {
                        eprintln!("   • Out of memory");
                    }
                    std::io::ErrorKind::Other => {
                        eprintln!("   • Other I/O error: {error}");
                    }
                    _ => {
                        eprintln!("   • File system error: {error}");
                    }
                }
            }
            CliError::SqliteError(error) => {
                eprintln!("💡 SQLite database error:");
                match error {
                    rusqlite::Error::SqliteFailure(code, msg) => match code.code {
                        rusqlite::ErrorCode::DatabaseBusy => {
                            eprintln!("   • Database is locked by another process");
                        }
                        rusqlite::ErrorCode::DatabaseLocked => {
                            eprintln!("   • Database is locked");
                        }
                        rusqlite::ErrorCode::ReadOnly => {
                            eprintln!("   • Database is read-only");
                        }
                        rusqlite::ErrorCode::DatabaseCorrupt => {
                            eprintln!("   • Database file is corrupted");
                        }
                        rusqlite::ErrorCode::ConstraintViolation => {
                            eprintln!("   • Database constraint violation");
                        }
                        rusqlite::ErrorCode::NotADatabase => {
                            eprintln!("   • File is not a valid SQLite database");
                        }
                        rusqlite::ErrorCode::CannotOpen => {
                            eprintln!("   • Cannot open database file");
                        }
                        _ => {
                            eprintln!(
                                "   • SQLite error ({}): {}",
                                code.code as u32,
                                msg.as_deref().unwrap_or("No additional details")
                            );
                        }
                    },
                    rusqlite::Error::InvalidColumnName(name) => {
                        eprintln!("   • Invalid column name: '{name}'");
                    }
                    rusqlite::Error::InvalidColumnIndex(index) => {
                        eprintln!("   • Invalid column index: {index}");
                    }
                    rusqlite::Error::InvalidColumnType(index, name, ty) => {
                        eprintln!(
                            "   • Type mismatch for column {index} ('{name}'): expected different type than {ty:?}"
                        );
                    }
                    rusqlite::Error::StatementChangedRows(expected) => {
                        eprintln!(
                            "   • Expected to change {expected} row(s), but operation affected different number",
                        );
                    }
                    rusqlite::Error::InvalidPath(path) => {
                        eprintln!("   • Invalid database path: {}", path.display());
                    }
                    rusqlite::Error::SqlInputError { error, .. } => {
                        eprintln!("   • SQL syntax error: {error}");
                    }
                    _ => {
                        eprintln!("   • Database error: {error}");
                    }
                }
            }
            CliError::HashingError(error) => {
                eprintln!("💡 Password hashing failed:");
                match error {
                    password_hash::Error::Algorithm => {
                        eprintln!("   • Unsupported or invalid algorithm");
                    }
                    password_hash::Error::B64Encoding(_) => {
                        eprintln!("   • Hash or salt contains invalid base64 characters");
                    }
                    password_hash::Error::Crypto => {
                        eprintln!("   • Cryptographic operation failed");
                    }
                    password_hash::Error::OutputSize { .. } => {
                        eprintln!("   • Invalid output size specified");
                    }
                    password_hash::Error::ParamNameDuplicated => {
                        eprintln!("   • Duplicate parameter name in hash string");
                    }
                    password_hash::Error::ParamNameInvalid => {
                        eprintln!("   • Invalid parameter name");
                    }
                    password_hash::Error::ParamValueInvalid(_) => {
                        eprintln!("   • Invalid parameter value");
                    }
                    password_hash::Error::Password => {
                        eprintln!("   • Password format is invalid");
                    }
                    password_hash::Error::PhcStringField => {
                        eprintln!("   • Invalid PHC string field");
                    }
                    password_hash::Error::PhcStringTrailingData => {
                        eprintln!("   • Unexpected trailing data in hash string");
                    }
                    password_hash::Error::SaltInvalid(_) => {
                        eprintln!("   • Invalid salt format or length");
                    }
                    password_hash::Error::Version => {
                        eprintln!("   • Unsupported algorithm version");
                    }
                    _ => {
                        eprintln!("   • Password hashing error: {error:?}");
                    }
                }
            }
            CliError::ConfigError(error) => {
                eprintln!("⚙️  Configuration error occurred:");
                eprintln!("   • Error details: {error}");
                eprintln!("   • Check if your config.yml file exists and is valid");
            }
            CliError::Other(error) => {
                eprintln!("💡 Unexpected error occurred:");
                eprintln!("   • Error details: {error}");
            }
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::MetaParseError(_) | CliError::HashingError(_) | CliError::ConfigError(_) => {
                10
            }
            CliError::MetaOpenError(_) => 11,
            CliError::SqliteError(_) => 12,
            CliError::Other(_) => 99,
        }
    }

    pub fn exit_with_tips(self) -> ! {
        eprintln!("❌ {self}");
        self.print_tip();
        std::process::exit(self.exit_code());
    }
}

pub trait ExitOnError<T> {
    fn or_exit(self) -> T;
}

impl<T> ExitOnError<T> for Result<T, CliError> {
    fn or_exit(self) -> T {
        self.unwrap_or_else(|err| err.exit_with_tips())
    }
}
impl From<password_hash::Error> for CliError {
    fn from(err: password_hash::Error) -> Self {
        CliError::HashingError(err)
    }
}
