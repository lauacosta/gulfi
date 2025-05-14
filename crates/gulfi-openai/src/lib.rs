use std::{
    env::var,
    time::{Duration, Instant},
};

use bytes::BufMut;
use color_eyre::owo_colors::OwoColorize;
use eyre::Result;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument, warn};

const MAX_INTENTOS: u32 = 5;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncodingFormat {
    Float,
    Base64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseBody {
    #[serde(rename = "data")]
    pub embeddings: Vec<EmbeddingObject>,
    // pub usage: TokenUsage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmbeddingObject {
    embedding: Vec<f32>,
}

impl EmbeddingObject {
    pub fn embeddings_iter(
        objects: impl IntoIterator<Item = Self>,
    ) -> impl Iterator<Item = Vec<f32>> {
        objects.into_iter().map(|obj| obj.embedding)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RequestBody {
    pub input: Vec<String>,
    pub model: String,
    pub encoding_format: Option<EncodingFormat>,
    pub dimensions: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Request falló: {0} {1}")]
    RequestError(reqwest::Error, String),
    #[error("Rate limit excecido")]
    RateLimit,
    #[error("Maximo número de intentos excedido")]
    MaxRetriesExceeded,
}

impl From<reqwest::Error> for EmbeddingError {
    fn from(err: reqwest::Error) -> Self {
        EmbeddingError::RequestError(err, String::default())
    }
}

async fn request_embeddings(
    client: &Client,
    token: &str,
    request: &RequestBody,
    attempt: u32,
    max_retries: u32,
    time_backoff: u64,
    proc_id: usize,
) -> Result<reqwest::Response, EmbeddingError> {
    if attempt > 0 {
        warn!("Intento {attempt}/{max_retries} [{proc_id}]");

        let base_delay = time_backoff * 2u64.pow(attempt);
        let jittered_delay = rand::rng().random_range(0..=base_delay / 2);

        debug!(%time_backoff, %base_delay, %jittered_delay);
        tokio::time::sleep(Duration::from_millis(base_delay + jittered_delay)).await;
    }

    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(token)
        .json(&request)
        .send()
        .await?;

    let status = response.status();

    match status.as_u16() {
        200..299 => Ok(response),
        429 | 502 | 520 => {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            if attempt >= max_retries {
                error!(
                    "El maximo numero de intentos fue excedido debido al Rate limit [{proc_id}]"
                );
                Err(EmbeddingError::MaxRetriesExceeded)
            } else {
                warn!("Rate limit excedido, volviendo a intentar... [{proc_id}]");

                if let Some(retry_after) = retry_after {
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                }

                Err(EmbeddingError::RateLimit)
            }
        }
        _ => {
            let error = response
                .error_for_status_ref()
                .expect_err("Deberia poder obtener el error");

            let err_body = response
                .text()
                .await
                .unwrap_or_else(|_| "No response body".to_string());

            let error_msg = format!("{status} -> {err_body}",);

            error!("El request ha fallado con status: {status}. Respuesta: {err_body} [{proc_id}]");

            Err(EmbeddingError::RequestError(error, error_msg))
        }
    }
}

// https://community.openai.com/t/does-the-index-field-on-an-embedding-response-correlate-to-the-index-of-the-input-text-it-was-generated-from/526099
// FIX: Siempre hay una request que devuelve 400 no 429.
pub async fn embed_vec_with_progress(
    indices: Vec<u64>,
    input: Vec<String>,
    client: &Client,
    proc_id: usize,
    base_delay: u64,
    tx: Sender<String>,
) -> Result<(Vec<(u64, Vec<f32>)>, u128)> {
    let global_start = Instant::now();

    let _ = tx
        .send(format!(
            "Preparando embedding para {} registros",
            input.len()
        ))
        .await;

    let request = RequestBody {
        input,
        model: "text-embedding-3-small".to_owned(),
        encoding_format: Some(EncodingFormat::Float),
        dimensions: Some(1536),
    };

    let open_ai_key =
        var("OPENAI_KEY").expect("No se encuentra la variable de entorno 'OPENAI_KEY'");

    let mut intento = 0;
    let mut response = None;

    while intento <= MAX_INTENTOS {
        let req_start = Instant::now();
        let _ = tx
            .send(format!(
                "Enviando request a OpenAI (intento {}/{})",
                intento + 1,
                MAX_INTENTOS + 1
            ))
            .await;
        match request_embeddings(
            client,
            &open_ai_key,
            &request,
            intento,
            MAX_INTENTOS,
            base_delay,
            proc_id,
        )
        .await
        {
            Ok(resp) => {
                let elapsed = req_start.elapsed().as_millis();
                let _ = tx.send(format!("Request exitoso en {} ms", elapsed)).await;
                response = Some(resp);
                break;
            }
            Err(EmbeddingError::RateLimit) => {
                let _ = tx
                    .send(format!(
                        "{} Rate limit alcanzado, reintentando ({}/{})",
                        "⚠️".bright_yellow(),
                        intento + 1,
                        MAX_INTENTOS + 1
                    ))
                    .await;
                intento += 1;
            }
            Err(e) => {
                let _ = tx.send(format!("{} Error: {}", "❌".bright_red(), e)).await;
                return Err(e.into());
            }
        }
    }

    let mut response = match response {
        Some(r) => r,
        None => {
            let _ = tx
                .send(format!("{} Máximo de intentos excedido", "❌".bright_red()))
                .await;
            return Err(EmbeddingError::MaxRetriesExceeded.into());
        }
    };

    let _ = tx.send("Deserializando respuesta...".to_string()).await;
    let start = Instant::now();

    let capacity = response.content_length().unwrap_or(0) as usize;
    let mut payload = Vec::with_capacity(capacity);
    while let Some(chunk) = response.chunk().await? {
        payload.put(chunk);
    }

    // let bytes = response.bytes().await?;
    // let mut buf = BytesMut::from(bytes);
    let response: ResponseBody = simd_json::serde::from_slice(&mut payload)?;

    let elapsed = start.elapsed().as_millis();
    let _ = tx
        .send(format!("Deserialización completada en {} ms", elapsed))
        .await;

    let _ = tx.send("Procesando embeddings...".to_string()).await;
    let embedding: Vec<(u64, Vec<f32>)> = std::iter::zip(
        indices,
        EmbeddingObject::embeddings_iter(response.embeddings),
    )
    .collect();

    let total_elapsed = global_start.elapsed().as_millis();
    let _ = tx
        .send(format!("Embeddings completados en ({}) ms", total_elapsed))
        .await;

    Ok((embedding, total_elapsed))
}

#[instrument(name = "Generando embedding a partir del query", skip(input, client))]
pub async fn embed_single(input: String, client: &Client) -> Result<Vec<f32>> {
    let global_start = Instant::now();

    #[derive(Serialize, Deserialize)]
    pub struct RequestBody {
        pub input: String,
        pub model: String,
        pub encoding_format: Option<EncodingFormat>,
        pub dimensions: Option<u64>,
    }

    let request = RequestBody {
        input,
        model: "text-embedding-3-small".to_owned(),
        encoding_format: Some(EncodingFormat::Float),
        dimensions: Some(1536),
    };

    let open_ai_key =
        var("OPENAI_KEY").expect("No se encuentra la variable de entorno 'OPENAI_KEY'");

    let req_start = Instant::now();
    info!("Enviando request a Open AI...");
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(open_ai_key)
        .json(&request)
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);
    info!("El request tomó {} ms", req_start.elapsed().as_millis());

    let start = Instant::now();
    let response: ResponseBody = response.json().await?;
    info!(
        "Deserializar la response a ResponseBody tomó {} ms",
        start.elapsed().as_millis()
    );

    let embedding = response
        .embeddings
        .into_iter()
        .next()
        .expect("Deberia tener minimo un elemento")
        .embedding;

    info!(
        "Embedding generado correctamente! en total tomó {} ms",
        global_start.elapsed().as_millis()
    );

    Ok(embedding)
}

// TODO: Implementar las interfaces para poder realizar batch requests y ahorrar gastos.
// pub async fn batch_embed(input: [&str]) -> eyre::Result<Vec<Vec<f32>>> {
// }

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
