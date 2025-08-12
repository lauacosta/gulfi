use std::{collections::BTreeMap, fmt::Display};
use thiserror::Error;

#[derive(Clone, PartialEq, Debug)]
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
    #[error("Search doesn't have 'query' key.")]
    MissingQuery,
    #[error("Search is empty.")]
    EmptyInput,
    #[error("No value after '{0}'")]
    MissingValue(char),
    #[error("No value before '{0}'")]
    MissingKey(char),
    #[error("Invalid token: '{0}'")]
    InvalidToken(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub query: String,
    pub constraints: Option<BTreeMap<String, Vec<Constraint>>>,
}

impl Query {
    // TODO: Write a good one
    pub fn parse(input: &str) -> Result<Self, ParsingError> {
        let input_clean = clean_html(input.to_owned());

        if input_clean.trim().is_empty() {
            return Err(ParsingError::EmptyInput);
        }

        if input_clean.chars().any(char::is_control) {
            return Err(ParsingError::InvalidToken(input_clean));
        }

        let input = input_clean.trim();

        let (query_part, constraints_part) = Self::split_query_and_constraints(input)?;

        let constraints = if let Some(constraints_str) = constraints_part {
            let parsed_constraints = Self::parse_constraints(constraints_str)?;

            if parsed_constraints.is_empty() {
                None
            } else {
                Some(parsed_constraints)
            }
        } else {
            None
        };

        Ok(Self {
            query: query_part,
            constraints,
        })
    }

    fn split_query_and_constraints(input: &str) -> Result<(String, Option<&str>), ParsingError> {
        if let Some((left, right)) = input.split_once(',') {
            let query = Self::extract_query_value(left)?;
            Ok((query, Some(right)))
        } else {
            // No comma - could be "query:value" or just "value"
            let query = Self::extract_query_value(input).unwrap_or_else(|_| input.to_string());
            Ok((query, None))
        }
    }

    fn extract_query_value(input: &str) -> Result<String, ParsingError> {
        match input.trim().split_once(':') {
            Some(("query", value)) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Err(ParsingError::MissingQuery)
                } else {
                    Ok(trimmed.to_string())
                }
            }
            _ => Err(ParsingError::MissingQuery),
        }
    }

    fn parse_constraints(
        constraints_str: &str,
    ) -> Result<BTreeMap<String, Vec<Constraint>>, ParsingError> {
        let mut constraints = BTreeMap::new();

        for token in constraints_str
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            let constraint = Self::parse_single_constraint(token)?;
            Self::add_constraint(&mut constraints, constraint);
        }

        Ok(constraints)
    }

    fn parse_single_constraint(token: &str) -> Result<(String, Constraint), ParsingError> {
        // Try each operator in order
        if let Some((key, value)) = token.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            if key.is_empty() {
                return Err(ParsingError::MissingKey(':'));
            }
            if value.is_empty() {
                return Err(ParsingError::MissingValue(':'));
            }

            return Ok((key.to_string(), Constraint::Exact(value.to_string())));
        }

        if let Some((key, value)) = token.split_once('<') {
            let key = key.trim();
            let value = value.trim();

            if key.is_empty() {
                return Err(ParsingError::MissingKey('<'));
            }
            if value.is_empty() {
                return Err(ParsingError::MissingValue('<'));
            }

            return Ok((key.to_string(), Constraint::LesserThan(value.to_string())));
        }

        if let Some((key, value)) = token.split_once('>') {
            let key = key.trim();
            let value = value.trim();

            if key.is_empty() {
                return Err(ParsingError::MissingKey('>'));
            }
            if value.is_empty() {
                return Err(ParsingError::MissingValue('>'));
            }

            return Ok((key.to_string(), Constraint::GreaterThan(value.to_string())));
        }

        Err(ParsingError::InvalidToken(token.to_string()))
    }

    fn add_constraint(
        constraints: &mut BTreeMap<String, Vec<Constraint>>,
        (key, constraint): (String, Constraint),
    ) {
        constraints.entry(key).or_default().push(constraint);
    }
}

#[cfg(test)]
mod tests;

#[inline]
pub fn clean_html(str: String) -> String {
    if ammonia::is_html(&str) {
        ammonia::clean(&str)
    } else {
        str
    }
}
