use color_eyre::owo_colors::OwoColorize;

#[derive(Clone, Debug)]
pub enum EmbeddingMessage {
    Preparing { count: usize },
    SendingRequest { attempt: usize, max_attempts: usize },
    RequestSuccessful { elapsed_ms: u128 },
    RateLimit { attempt: usize, max_attempts: usize },
    Error { message: String },
    MaxRetriesExceeded,
    ParsingResponse,
    ParsingComplete { elapsed_ms: u128 },
    ProcessingEmbeddings,
    Complete { total_elapsed_ms: u128 },
}
impl std::fmt::Display for EmbeddingMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preparing { count } => write!(f, "Preparing embeddings for {count} entries"),
            Self::SendingRequest {
                attempt,
                max_attempts,
            } => {
                write!(f, "Sending request. (intento {attempt}/{max_attempts})")
            }
            Self::RequestSuccessful { elapsed_ms } => {
                write!(f, "Request successful {elapsed_ms} ms")
            }
            Self::RateLimit {
                attempt,
                max_attempts,
            } => {
                write!(
                    f,
                    "{} Rate limit hit, trying again ({attempt}/{max_attempts})...",
                    "⚠️".bright_yellow()
                )
            }
            Self::Error { message } => {
                write!(f, "{} Error: {message}", "❌".bright_red())
            }
            Self::MaxRetriesExceeded => {
                write!(f, "{} Max retries exceeded", "❌".bright_red())
            }
            Self::ParsingResponse => write!(f, "Parsing response..."),
            Self::ParsingComplete { elapsed_ms } => {
                write!(f, "Parsing response done in {elapsed_ms} ms")
            }
            Self::ProcessingEmbeddings => write!(f, "Processing embeddings..."),
            Self::Complete { total_elapsed_ms } => {
                write!(f, "Embeddings done in ({total_elapsed_ms}) ms")
            }
        }
    }
}
