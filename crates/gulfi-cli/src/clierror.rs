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
                        eprintln!("   • Check if all required fields are present");
                        eprintln!("   • Verify field names and types are correct");
                    }
                    serde_json::Error { .. } if error.is_eof() => {
                        eprintln!("   • Unexpected end of file");
                        eprintln!("   • metadata file appears to be incomplete");
                        eprintln!("   • Check if the file was truncated during save");
                        eprintln!("   • Ensure all brackets and braces are properly closed");
                    }
                    serde_json::Error { .. } if error.is_io() => {
                        eprintln!("   • I/O error while reading JSON: {error}");
                        eprintln!("   • Check file permissions and disk space");
                        eprintln!("   • Ensure the file is not locked by another process");
                    }
                    _ => {
                        eprintln!("   • JSON parsing error: {error}");
                        eprintln!("   • Check if metadata file contains valid JSON syntax");
                        eprintln!("   • You may need to recreate metadata file if it's corrupted");
                    }
                }
            }
            CliError::MetaOpenError(error) => {
                eprintln!("💡 Cannot open metadata file file:");
                match error.kind() {
                    std::io::ErrorKind::NotFound => {
                        eprintln!("   • metadata file file not found");
                        eprintln!("   • Make sure you're in the correct directory");
                        eprintln!("   • Create metadata file if it doesn't exist");
                        eprintln!("   • Check if the file name is spelled correctly");
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        eprintln!("   • Permission denied - check file permissions");
                        eprintln!("   • Try: chmod 644 metadata file");
                        eprintln!("   • Ensure you have read access to the file");
                        eprintln!("   • Check if the file is owned by another user");
                    }
                    std::io::ErrorKind::AlreadyExists => {
                        eprintln!("   • File already exists (unexpected in this context)");
                        eprintln!("   • This might indicate a logic error in the application");
                    }
                    std::io::ErrorKind::WouldBlock => {
                        eprintln!("   • File operation would block");
                        eprintln!("   • The file might be locked by another process");
                        eprintln!("   • Try closing other applications and retry");
                    }
                    std::io::ErrorKind::InvalidInput => {
                        eprintln!("   • Invalid file path or name");
                        eprintln!("   • Check for invalid characters in the path");
                        eprintln!("   • Ensure the path format is correct for your OS");
                    }
                    std::io::ErrorKind::InvalidData => {
                        eprintln!("   • File contains invalid data");
                        eprintln!("   • The file might be corrupted");
                        eprintln!("   • Try recreating metadata file");
                    }
                    std::io::ErrorKind::TimedOut => {
                        eprintln!("   • File operation timed out");
                        eprintln!("   • Network drive or slow storage device");
                        eprintln!("   • Try again or check storage performance");
                    }
                    std::io::ErrorKind::Interrupted => {
                        eprintln!("   • File operation was interrupted");
                        eprintln!("   • Try the operation again");
                    }
                    std::io::ErrorKind::OutOfMemory => {
                        eprintln!("   • Out of memory");
                        eprintln!("   • Close other applications to free memory");
                        eprintln!("   • Check if the file is unusually large");
                    }
                    std::io::ErrorKind::Other => {
                        eprintln!("   • Other I/O error: {error}");
                        eprintln!("   • Check disk space and file system integrity");
                        eprintln!("   • Ensure the storage device is functioning properly");
                    }
                    _ => {
                        eprintln!("   • File system error: {error}");
                        eprintln!("   • Check disk space and file system integrity");
                        eprintln!("   • Ensure the file is not locked by another process");
                    }
                }
            }
            CliError::SqliteError(error) => {
                eprintln!("💡 SQLite database error:");
                match error {
                    rusqlite::Error::SqliteFailure(code, msg) => match code.code {
                        rusqlite::ErrorCode::DatabaseBusy => {
                            eprintln!("   • Database is locked by another process");
                            eprintln!("   • Close any other instances of the application");
                            eprintln!("   • Wait a moment and try again");
                        }
                        rusqlite::ErrorCode::DatabaseLocked => {
                            eprintln!("   • Database is locked");
                            eprintln!("   • Another transaction may be in progress");
                            eprintln!("   • Close other database connections and retry");
                        }
                        rusqlite::ErrorCode::ReadOnly => {
                            eprintln!("   • Database is read-only");
                            eprintln!("   • Check file permissions: chmod 644 database.db");
                            eprintln!("   • Ensure the directory is writable");
                        }
                        rusqlite::ErrorCode::DatabaseCorrupt => {
                            eprintln!("   • Database file is corrupted");
                            eprintln!("   • Consider restoring from backup");
                            eprintln!("   • You may need to recreate the database");
                        }
                        rusqlite::ErrorCode::ConstraintViolation => {
                            eprintln!("   • Database constraint violation");
                            eprintln!("   • Check for duplicate or invalid data");
                            eprintln!("   • Verify required fields are provided");
                        }
                        rusqlite::ErrorCode::NotADatabase => {
                            eprintln!("   • File is not a valid SQLite database");
                            eprintln!("   • Check if the file was corrupted during transfer");
                            eprintln!("   • You may need to recreate the database");
                        }
                        rusqlite::ErrorCode::CannotOpen => {
                            eprintln!("   • Cannot open database file");
                            eprintln!("   • Check file permissions and path");
                            eprintln!("   • Ensure directory exists and is writable");
                        }
                        _ => {
                            eprintln!(
                                "   • SQLite error ({}): {}",
                                code.code as u32,
                                msg.as_deref().unwrap_or("No additional details")
                            );
                            eprintln!("   • Check database file integrity");
                            eprintln!("   • Ensure sufficient disk space");
                        }
                    },
                    rusqlite::Error::InvalidColumnName(name) => {
                        eprintln!("   • Invalid column name: '{name}'");
                        eprintln!("   • Check your SQL query for typos");
                        eprintln!("   • Verify the database schema matches expectations");
                    }
                    rusqlite::Error::InvalidColumnIndex(index) => {
                        eprintln!("   • Invalid column index: {index}");
                        eprintln!("   • Column index is out of bounds");
                        eprintln!("   • Check your query result handling");
                    }
                    rusqlite::Error::InvalidColumnType(index, name, ty) => {
                        eprintln!(
                            "   • Type mismatch for column {index} ('{name}'): expected different type than {ty:?}"
                        );
                        eprintln!("   • Check data types in your query");
                        eprintln!("   • Verify column contains expected data type");
                    }
                    rusqlite::Error::StatementChangedRows(expected) => {
                        eprintln!(
                            "   • Expected to change {expected} row(s), but operation affected different number",
                        );
                        eprintln!("   • This might indicate data inconsistency");
                        eprintln!("   • Check your WHERE clauses and data");
                    }
                    rusqlite::Error::InvalidPath(path) => {
                        eprintln!("   • Invalid database path: {}", path.display());
                        eprintln!("   • Check if the path exists and is accessible");
                        eprintln!("   • Verify file permissions");
                    }
                    rusqlite::Error::SqlInputError { error, .. } => {
                        eprintln!("   • SQL syntax error: {error}");
                        eprintln!("   • Check your SQL query for syntax issues");
                        eprintln!("   • Verify table and column names");
                    }
                    _ => {
                        eprintln!("   • Database error: {error}");
                        eprintln!("   • Check database file integrity");
                        eprintln!("   • Ensure sufficient disk space");
                        eprintln!("   • Try recreating the database if issue persists");
                    }
                }
            }
            CliError::HashingError(error) => {
                eprintln!("💡 Password hashing failed:");
                match error {
                    password_hash::Error::Algorithm => {
                        eprintln!("   • Unsupported or invalid algorithm");
                        eprintln!("   • Check if the specified algorithm is available");
                        eprintln!("   • Supported algorithms: Argon2, bcrypt, scrypt, pbkdf2");
                        eprintln!("   • Verify algorithm name spelling and case");
                    }
                    password_hash::Error::B64Encoding(_) => {
                        eprintln!("   • Base64 encoding error");
                        eprintln!("   • Hash or salt contains invalid base64 characters");
                        eprintln!("   • Check if the hash string was corrupted");
                        eprintln!("   • Try regenerating the hash");
                    }
                    password_hash::Error::Crypto => {
                        eprintln!("   • Cryptographic operation failed");
                        eprintln!("   • Internal crypto library error");
                        eprintln!("   • This may indicate insufficient system resources");
                        eprintln!("   • Try again or use different parameters");
                    }
                    password_hash::Error::OutputSize { .. } => {
                        eprintln!("   • Invalid output size specified");
                        eprintln!("   • Hash output length is not supported");
                        eprintln!("   • Check algorithm-specific output size limits");
                        eprintln!("   • Use default output size if unsure");
                    }
                    password_hash::Error::ParamNameDuplicated => {
                        eprintln!("   • Duplicate parameter name in hash string");
                        eprintln!("   • PHC string format is malformed");
                        eprintln!("   • Check for repeated parameter names");
                        eprintln!("   • Regenerate the hash string");
                    }
                    password_hash::Error::ParamNameInvalid => {
                        eprintln!("   • Invalid parameter name");
                        eprintln!("   • Parameter name doesn't match algorithm specification");
                        eprintln!("   • Check parameter names in hash string");
                        eprintln!("   • Refer to algorithm documentation");
                    }
                    password_hash::Error::ParamValueInvalid(_) => {
                        eprintln!("   • Invalid parameter value");
                        eprintln!("   • Parameter value is out of acceptable range");
                        eprintln!("   • Check cost/iteration parameters");
                        eprintln!("   • Use recommended values for your algorithm");
                    }
                    password_hash::Error::Password => {
                        eprintln!("   • Password format is invalid");
                        eprintln!("   • Password may contain unsupported characters");
                        eprintln!("   • Check for null bytes or control characters");
                        eprintln!("   • Ensure password is properly UTF-8 encoded");
                    }
                    password_hash::Error::PhcStringField => {
                        eprintln!("   • Invalid PHC string field");
                        eprintln!("   • Hash string format is malformed");
                        eprintln!("   • Check if hash follows $algorithm$params$salt$hash format");
                        eprintln!("   • Verify all required fields are present");
                    }
                    password_hash::Error::PhcStringTrailingData => {
                        eprintln!("   • Unexpected trailing data in hash string");
                        eprintln!("   • Hash string has extra characters at the end");
                        eprintln!("   • Remove any trailing whitespace or characters");
                        eprintln!("   • Ensure hash string ends properly");
                    }
                    password_hash::Error::SaltInvalid(_) => {
                        eprintln!("   • Invalid salt format or length");
                        eprintln!("   • Salt must be properly base64 encoded");
                        eprintln!("   • Check if salt meets minimum length requirements");
                        eprintln!("   • Try regenerating a new random salt");
                    }
                    password_hash::Error::Version => {
                        eprintln!("   • Unsupported algorithm version");
                        eprintln!("   • Algorithm version in hash string is not supported");
                        eprintln!("   • Check if you're using a compatible version");
                        eprintln!("   • Consider upgrading or using a different version");
                    }
                    _ => {
                        eprintln!("   • Password hashing error: {error:?}");
                        eprintln!("   • Check if all hashing parameters are valid");
                        eprintln!("   • Verify the password and salt format");
                        eprintln!("   • Consider trying a different hashing algorithm");
                    }
                }
            }
            CliError::ConfigError(error) => {
                eprintln!("⚙️  Configuration error occurred:");
                eprintln!("   • Error details: {error}");
                eprintln!("   • Check if your config.yml file exists and is valid");
                eprintln!("   • Verify all required configuration fields are present");
                eprintln!("   • Ensure YAML syntax is correct (proper indentation, no tabs)");
                eprintln!("   • Check file permissions for configuration directory");
                eprintln!("   • Try regenerating the config file if corrupted");
                eprintln!("   • Common fixes:");
                eprintln!("     - Run `gulfi init` to create a new config");
                eprintln!("     - Validate YAML syntax online or with a YAML linter");
                eprintln!("     - Check for missing quotes around string values");
            }
            CliError::Other(error) => {
                eprintln!("💡 Unexpected error occurred:");
                eprintln!("   • Error details: {error}");
                eprintln!("   • This may be a bug in the application");
                eprintln!("   • Try running the command again");
                eprintln!("   • Check if all required dependencies are installed");
                eprintln!("   • Report this issue if it persists:");
                eprintln!("     - Include the full error message");
                eprintln!("     - Describe what you were trying to do");
                eprintln!("     - Mention your operating system and version");
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
