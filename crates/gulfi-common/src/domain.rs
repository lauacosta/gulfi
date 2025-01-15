use crate::normalize;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Clone, Default, Deserialize, Debug)]
pub struct TneaData {
    pub email: String,
    pub nombre: String,
    #[serde(deserialize_with = "default_if_empty")]
    pub sexo: String,
    pub fecha_nacimiento: String,
    #[serde(deserialize_with = "deserialize_number_from_string_including_empty")]
    pub edad: usize,
    pub provincia: String,
    pub ciudad: String,
    #[serde(deserialize_with = "default_if_empty")]
    pub descripcion: String,
    #[serde(deserialize_with = "default_if_empty")]
    pub estudios: String,
    #[serde(deserialize_with = "default_if_empty")]
    pub experiencia: String,
    #[serde(deserialize_with = "default_if_empty")]
    pub estudios_mas_recientes: String,
}

// https://serde.rs/field-attrs.html#deserialize_with
fn default_if_empty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.unwrap_or_default())
}

fn deserialize_number_from_string_including_empty<'de, D>(
    deserializer: D,
) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) if s.is_empty() => Ok(0),
        serde_json::Value::String(s) => s.parse::<usize>().map_err(serde::de::Error::custom),
        serde_json::Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("Invalid number format"))
            .map(|n| n as usize),
        serde_json::Value::Null => Ok(0),
        _ => Err(serde::de::Error::custom("Expected string or number")),
    }
}

#[derive(Debug)]
pub struct SearchString {
    pub query: String,
    pub provincia: Option<String>,
    pub ciudad: Option<String>,
}

impl SearchString {
    pub fn parse(search_str: &str) -> Self {
        if let Some((query, filters)) = search_str.split_once('|') {
            if let Some((provincia, ciudad)) = filters.split_once(',') {
                // TODO: Filter if it is only whitespace.
                let provincia = Some(format!("%{}%", normalize(provincia)));
                let ciudad = Some(format!("%{}%", normalize(ciudad)));
                Self {
                    query: normalize(query),
                    provincia,
                    ciudad,
                }
            } else {
                let provincia = Some(format!("%{}%", normalize(filters)));
                Self {
                    query: normalize(query),
                    provincia,
                    ciudad: None,
                }
            }
        } else {
            Self {
                query: normalize(search_str),
                provincia: None,
                ciudad: None,
            }
        }
    }
}
