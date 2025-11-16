use super::*;
use expect_test::{Expect, expect};
use proptest::prelude::*;

fn check(query: &str, expect: Expect) {
    let res = Query::parse(query);
    expect.assert_debug_eq(&res);
}

#[test]
fn fails_gracefully_on_control_characters() {
    check(
        "query: Test,;\0",
        expect![[r#"
        Err(
            InvalidToken(
                ";",
            ),
        )
    "#]],
    );
}

#[test]
fn only_query() {
    check(
        "query: Lautaro,",
        expect![[r#"
            Ok(
                Query {
                    query: "Lautaro",
                    constraints: None,
                },
            )
        "#]],
    );
}

#[test]
fn only_with_filters() {
    check(
        "query: Lautaro, ciudad: Corrientes, provincia: Mendoza",
        expect![[r#"
            Ok(
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
                },
            )
        "#]],
    );
}

#[test]
fn only_with_restrictions() {
    check(
        "query: lautaro, edad > 30, edad < 60",
        expect![[r#"
            Ok(
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
                },
            )
        "#]],
    );
}

#[test]
fn with_all_constraints() {
    check(
        "query: Lautaro, ciudad: Corrientes, provincia: Mendoza, edad > 30, edad < 60",
        expect![[r#"
            Ok(
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
                },
            )
        "#]],
    );
}

#[test]
fn fails_without_query_prefix() {
    check(
        "Lautaro, age > 30",
        expect![[r#"
        Err(
            MissingQuery,
        )
    "#]],
    );
}

#[test]
fn fails_with_empty_query() {
    check(
        "query: , age > 30",
        expect![[r#"
        Err(
            MissingQuery,
        )
    "#]],
    );
}

#[test]
fn fails_with_no_colon_to_split_query() {
    check(
        "query Lautaro, age > 30",
        expect![[r#"
        Err(
            MissingQuery,
        )
    "#]],
    );
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
    check(
        "query: Lautaro, :Berlin",
        expect![[r#"
        Err(
            MissingKey(
                ':',
            ),
        )
    "#]],
    );

    check(
        "query: Lautaro, >30",
        expect![[r#"
        Err(
            MissingKey(
                '>',
            ),
        )
    "#]],
    );

    check(
        "query: Lautaro, <30",
        expect![[r#"
        Err(
            MissingKey(
                '<',
            ),
        )
    "#]],
    );
}

#[test]
fn fails_with_invalid_token() {
    check(
        "query: Lautaro, city; Corrientes",
        expect![[r#"
        Err(
            InvalidToken(
                "city; Corrientes",
            ),
        )
    "#]],
    );
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
