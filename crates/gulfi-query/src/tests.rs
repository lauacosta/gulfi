use super::*;
use expect_test::{Expect, expect};
use pretty_assertions::assert_eq;
use proptest::prelude::*;

fn check(query: &str, expect: Expect) {
    let res = Query::parse(query).unwrap();
    expect.assert_debug_eq(&res);
}

#[test]
fn fails_gracefully_on_control_characters() {
    let input = "query: Test, ;\0";
    let result = Query::parse(input);

    assert!(matches!(result, Err(ParsingError::InvalidToken(_))));
}

#[test]
fn only_query() {
    check(
        "query: Lautaro,",
        expect![[r#"
        Query {
            query: "Lautaro",
            constraints: None,
        }
    "#]],
    );
}

#[test]
fn only_with_filters() {
    check(
        "query: Lautaro, ciudad: Corrientes, provincia: Mendoza",
        expect![[r#"
            Query {
                query: "Lautaro",
                constraints: Some(
                    {
                        "ciudad": [
                            Exact(
                                "Corrientes",
                            ),
                        ],
                        "provincia": [
                            Exact(
                                "Mendoza",
                            ),
                        ],
                    },
                ),
            }
        "#]],
    );
}

#[test]
fn only_with_restrictions() {
    check(
        "query: lautaro, edad > 30, edad < 60",
        expect![[r#"
        Query {
            query: "lautaro",
            constraints: Some(
                {
                    "edad": [
                        GreaterThan(
                            "30",
                        ),
                        LesserThan(
                            "60",
                        ),
                    ],
                },
            ),
        }
    "#]],
    );
}

#[test]
fn with_all_constraints() {
    check(
        "query: Lautaro, ciudad: Corrientes, provincia: Mendoza, edad > 30, edad < 60",
        expect![[r#"
            Query {
                query: "Lautaro",
                constraints: Some(
                    {
                        "ciudad": [
                            Exact(
                                "Corrientes",
                            ),
                        ],
                        "edad": [
                            GreaterThan(
                                "30",
                            ),
                            LesserThan(
                                "60",
                            ),
                        ],
                        "provincia": [
                            Exact(
                                "Mendoza",
                            ),
                        ],
                    },
                ),
            }
        "#]],
    );
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
