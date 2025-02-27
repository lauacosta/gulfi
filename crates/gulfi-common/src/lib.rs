mod datasources;
mod domain;
mod into_http;

pub use datasources::*;
pub use domain::*;
pub use into_http::*;

#[inline]
pub fn normalize(str: &str) -> String {
    str.trim_matches(|c| !char::is_ascii_alphabetic(&c))
        .trim()
        .to_lowercase()
        .replace("province", "")
}

#[inline]
pub fn clean_html(str: String) -> String {
    if ammonia::is_html(&str) {
        ammonia::clean(&str)
    } else {
        str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_query_only() {
        let search = SearchString::parse("example query");
        assert_eq!(search.query, "example query");
        assert!(search.provincia.is_none());
        assert!(search.ciudad.is_none());
    }

    #[test]
    fn parse_with_query_and_provincia() {
        let search = SearchString::parse("example query|some provincia");
        assert_eq!(search.query, "example query");
        assert_eq!(search.provincia, Some("%some provincia%".to_owned()));
        assert!(search.ciudad.is_none());
    }

    #[test]
    fn parse_with_query_provincia_and_ciudad() {
        let search = SearchString::parse("example query|some provincia,some ciudad");
        assert_eq!(search.query, "example query");
        assert_eq!(search.provincia, Some("%some provincia%".to_owned()));
        assert_eq!(search.ciudad, Some("%some ciudad%".to_owned()));
    }

    #[test]
    fn parse_with_empty_filters() {
        let search = SearchString::parse("example query|");
        assert_eq!(search.query, "example query");
        assert_eq!(search.provincia, Some("%%".to_owned()));
        assert!(search.ciudad.is_none());
    }

    #[test]
    fn parse_with_empty_provincia() {
        let search = SearchString::parse("example query|,some ciudad");
        assert_eq!(search.query, "example query");
        assert_eq!(search.provincia, Some("%%".to_owned()));
        assert_eq!(search.ciudad, Some("%some ciudad%".to_owned()));
    }

    #[test]
    fn parse_with_extra_whitespace() {
        let search = SearchString::parse(" example query |  some provincia , some ciudad  ");
        assert_eq!(search.query, "example query");
        assert_eq!(search.provincia, Some("%some provincia%".to_owned()));
        assert_eq!(search.ciudad, Some("%some ciudad%".to_owned()));
    }

    #[test]
    fn parse_with_empty_input() {
        let search = SearchString::parse("");
        assert_eq!(search.query, "");
        assert!(search.provincia.is_none());
        assert!(search.ciudad.is_none());
    }

    #[test]
    fn parse_with_only_whitespace_input() {
        let search = SearchString::parse("   ");
        assert_eq!(search.query, "");
        assert!(search.provincia.is_none());
        assert!(search.ciudad.is_none());
    }
}
