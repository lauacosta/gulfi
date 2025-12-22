#[derive(Clone, Debug)]
pub struct EmbeddingEvent {
    pub proc_id: usize,
    pub kind: EmbeddingEventKind,
}

#[derive(Clone, Debug)]
pub enum EmbeddingEventKind {
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
