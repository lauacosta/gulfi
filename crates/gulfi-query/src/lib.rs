use std::{collections::HashMap, fmt::Display};
use thiserror::Error;

#[derive(PartialEq, Debug)]
pub enum Constraint {
    Exact(String),
    GreaterThan(String),
    LesserThan(String),
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Exact(str) => write!(f, ": {str}"),
            Constraint::GreaterThan(str) => write!(f, "> {str}"),
            Constraint::LesserThan(str) => write!(f, "< {str}"),
        }
    }
}

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Search doesnt have 'query' key.")]
    MissingQuery,
    #[error("No value after '{0}'")]
    MissingValue(char),
    #[error("No value before '{0}'")]
    MissingKey(char),
    #[error("Invalid token: '{0}'")]
    InvalidToken(String),
}

#[derive(Debug, PartialEq)]
pub struct Query {
    pub query: String,
    pub constraints: Option<HashMap<String, Vec<Constraint>>>,
}

impl Query {
    // TODO: Write a good one
    pub fn parse(input: &str) -> Result<Self, ParsingError> {
        let mut constraints: HashMap<String, Vec<Constraint>> = HashMap::new();

        let input_clean = clean_html(input.to_owned());

        if input_clean.chars().any(char::is_control) {
            return Err(ParsingError::InvalidToken(input_clean));
        }

        let input = input_clean.as_str();

        let (query, rest) = match input.split_once(',') {
            Some((lhs, rhs)) => {
                let query = match lhs.split_once(':') {
                    Some(("query", q)) if !q.trim().is_empty() => q.trim(),
                    _ => return Err(ParsingError::MissingQuery),
                };
                (query, Some(rhs))
            }
            None => (input, None),
        };

        if let Some(rhs) = rest {
            for token in rhs.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                if let Some((k, v)) = token.split_once(':') {
                    match (k, v) {
                        ("", _) => return Err(ParsingError::MissingKey(':')),
                        (_, "") => return Err(ParsingError::MissingValue(':')),
                        (k, v) => Self::update_constraints(
                            &mut constraints,
                            k.trim().to_owned(),
                            Constraint::Exact(v.trim().to_owned()),
                        ),
                    }
                } else if let Some((k, v)) = token.split_once('<') {
                    match (k, v) {
                        ("", _) => return Err(ParsingError::MissingKey('<')),
                        (_, "") => return Err(ParsingError::MissingValue('<')),
                        (k, v) => Self::update_constraints(
                            &mut constraints,
                            k.trim().to_owned(),
                            Constraint::LesserThan(v.trim().to_owned()),
                        ),
                    }
                } else if let Some((k, v)) = token.split_once('>') {
                    match (k, v) {
                        ("", _) => return Err(ParsingError::MissingKey('>')),
                        (_, "") => return Err(ParsingError::MissingValue('>')),
                        (k, v) => Self::update_constraints(
                            &mut constraints,
                            k.trim().to_owned(),
                            Constraint::GreaterThan(v.trim().to_owned()),
                        ),
                    }
                } else {
                    return Err(ParsingError::InvalidToken(token.to_string()));
                }
            }
        }

        Ok(Self {
            query: query.to_owned(),
            constraints: if constraints.is_empty() {
                None
            } else {
                Some(constraints)
            },
        })
    }

    fn update_constraints(
        constraints: &mut HashMap<String, Vec<Constraint>>,
        key: String,
        constraint: Constraint,
    ) {
        constraints.entry(key).or_default().push(constraint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    #[test]
    fn fails_gracefully_on_control_characters() {
        let input = "query: Test, ;\0";
        let result = Query::parse(input);
        dbg!(&result);

        assert!(matches!(result, Err(ParsingError::InvalidToken(_))));
    }

    #[test]
    fn only_query() {
        let result = Query::parse("query: Lautaro,").unwrap();

        assert_eq!(
            result,
            Query {
                query: "Lautaro".to_owned(),
                constraints: None,
            }
        )
    }

    #[test]
    fn only_with_filters() {
        let result =
            Query::parse("query: Lautaro, ciudad: Corrientes, provincia: Mendoza").unwrap();
        let mut expected = HashMap::new();
        expected.insert(
            "ciudad".to_owned(),
            vec![Constraint::Exact("Corrientes".to_owned())],
        );
        expected.insert(
            "provincia".to_owned(),
            vec![Constraint::Exact("Mendoza".to_owned())],
        );

        assert_eq!(
            result,
            Query {
                query: "Lautaro".to_owned(),
                constraints: Some(expected),
            }
        )
    }

    #[test]
    fn only_with_restrictions() {
        let result = Query::parse("query: Lautaro, edad > 30, edad < 60").unwrap();
        let mut expected = HashMap::new();
        expected.insert(
            "edad".to_owned(),
            vec![
                Constraint::GreaterThan("30".to_owned()),
                Constraint::LesserThan("60".to_owned()),
            ],
        );

        assert_eq!(
            result,
            Query {
                query: "Lautaro".to_owned(),
                constraints: Some(expected),
            }
        )
    }

    #[test]
    fn with_all_constraints() {
        let result = Query::parse(
            "query: Lautaro, ciudad: Corrientes, provincia: Mendoza, edad > 30, edad < 60",
        )
        .unwrap();

        let mut expected = HashMap::new();
        expected.insert(
            "edad".to_owned(),
            vec![
                Constraint::GreaterThan("30".to_owned()),
                Constraint::LesserThan("60".to_owned()),
            ],
        );
        expected.insert(
            "ciudad".to_owned(),
            vec![Constraint::Exact("Corrientes".to_owned())],
        );
        expected.insert(
            "provincia".to_owned(),
            vec![Constraint::Exact("Mendoza".to_owned())],
        );

        assert_eq!(
            result,
            Query {
                query: "Lautaro".to_owned(),
                constraints: Some(expected),
            }
        )
    }

    #[test]
    fn fails_without_query_prefix() {
        let result = Query::parse("Lautaro, age > 30");
        assert!(matches!(result, Err(ParsingError::MissingQuery)));
    }

    #[test]
    fn fails_with_empty_query() {
        let result = Query::parse("query: , age > 30");
        assert!(matches!(result, Err(ParsingError::MissingQuery)));
    }

    #[test]
    fn fails_with_no_colon_to_split_query() {
        let result = Query::parse("query Lautaro, age > 30");
        assert!(matches!(result, Err(ParsingError::MissingQuery)));
    }

    #[test]
    fn fails_with_token_missing_value() {
        let result = Query::parse("query: Lautaro, city:");
        assert!(matches!(result, Err(ParsingError::MissingValue(':'))));

        let result = Query::parse("query: Lautaro, edad>");
        assert!(matches!(result, Err(ParsingError::MissingValue('>'))));

        let result = Query::parse("query: Lautaro, edad<");
        assert!(matches!(result, Err(ParsingError::MissingValue('<'))));
    }

    #[test]
    fn fails_with_token_missing_key() {
        let result = Query::parse("query: Lautaro, :Berlin");
        assert!(matches!(result, Err(ParsingError::MissingKey(':'))));

        let result = Query::parse("query: Lautaro, >30");
        assert!(matches!(result, Err(ParsingError::MissingKey('>'))));

        let result = Query::parse("query: Lautaro, <30");
        assert!(matches!(result, Err(ParsingError::MissingKey('<'))));
    }

    #[test]
    fn fails_with_invalid_token() {
        let result = Query::parse("query: Lautaro, city; Corrientes");
        match result {
            Err(ParsingError::InvalidToken(token)) => assert_eq!(token, "city; Corrientes"),
            _ => panic!("Expected InvalidToken error"),
        }
    }

    proptest! {
      #[test]
      fn parses_valid_query_does_not_panic(query_str in generate_valid_query()) {
          let _ = Query::parse(&query_str);
      }

      #[test]
      fn fails_gracefully_on_bad_token(bad_token in "[^,:><]+;[^,:><]+") {
          let input = format!("query: Test, {}", bad_token);
          prop_assert!(matches!(Query::parse(&input), Err(ParsingError::InvalidToken(_))));
      }

    }

    fn generate_valid_query() -> impl Strategy<Value = String> {
        let word = "[a-zA-Z]+";
        let val = "[a-zA-Z0-9]+";

        let constraint = prop_oneof![
            (Just("query:".to_string()), any::<String>()).prop_map(|(k, v)| format!("{} {}", k, v)),
            (word, val).prop_map(|(k, v)| format!("{}: {}", k, v)),
            (word, val).prop_map(|(k, v)| format!("{} > {}", k, v)),
            (word, val).prop_map(|(k, v)| format!("{} < {}", k, v)),
        ];

        prop::collection::vec(constraint, 1..5).prop_map(|parts| parts.join(", "))
    }
}

#[inline]
pub fn clean_html(str: String) -> String {
    if ammonia::is_html(&str) {
        ammonia::clean(&str)
    } else {
        str
    }
}
