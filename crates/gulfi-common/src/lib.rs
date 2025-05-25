mod datasources;
pub use datasources::*;

pub const MEMORY_DB_PATH: &str = ":memory:";
pub const META_JSON_FILE: &str = "meta.json";
pub const MILLISECONDS_MULTIPLIER: u64 = 1000;
pub const SEPARATOR_LINE: &str = "----------------------------------------------------------------------------------------------------";

#[inline]
pub fn normalize(str: &str) -> String {
    str.trim_matches(|c| !char::is_ascii_alphabetic(&c))
        .trim()
        .to_lowercase()
}

#[inline]
pub fn clean_html(str: String) -> String {
    if ammonia::is_html(&str) {
        ammonia::clean(&str)
    } else {
        str
    }
}
