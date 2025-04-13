mod datasources;
mod into_http;

pub use datasources::*;
pub use into_http::*;

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
