mod reader;
mod sqlite;

use std::collections::VecDeque;
use std::io::Write;
use std::{
    collections::BTreeMap,
    sync::mpsc::{Receiver, Sender},
    thread::JoinHandle,
    time::Duration,
};

use color_eyre::owo_colors::OwoColorize;
use gulfi_types::embedding_events::{EmbeddingEvent, EmbeddingEventKind};
pub use reader::*;
pub use sqlite::*;

pub const MEMORY_DB_PATH: &str = ":memory:";
pub const SEPARATOR_LINE: &str = "----------------------------------------------------------------------------------------------------";

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

#[derive(Debug)]
pub struct SyncStatistics {
    pub total_inserted: usize,
    pub average: f32,
    pub time_elapsed: u128,
}

pub enum RenderEvent {
    SpinnerTick,
    Done,
}

pub fn spawn_spinner(render_tx: Sender<RenderEvent>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while render_tx.send(RenderEvent::SpinnerTick).is_ok() {
            std::thread::sleep(Duration::from_millis(100));
        }
    })
}

pub fn spawn_render_thread(message: String, render_rx: Receiver<RenderEvent>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let ticks = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut tick_index = 0;

        while let Ok(event) = render_rx.recv() {
            match event {
                RenderEvent::SpinnerTick => {
                    eprint!("\r{} {message}", ticks[tick_index]);
                    tick_index = (tick_index + 1) % ticks.len();
                }
                RenderEvent::Done => {
                    eprint!("\r\x1b[2K");
                    break;
                }
            }
            std::io::stderr()
                .flush()
                .expect("Couldn't flush stderr for some reason");
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChunkStatus {
    Preparing { count: usize },
    Sending { attempt: usize, max_attempts: usize },
    RateLimited { attempt: usize, max_attempts: usize },
    Parsing,
    Processing,
    Completed { elapsed_ms: u128 },
    Failed { reason: String },
}

impl From<EmbeddingEvent> for ChunkStatus {
    fn from(value: EmbeddingEvent) -> Self {
        match value.kind {
            EmbeddingEventKind::Preparing { count } => ChunkStatus::Preparing { count },
            EmbeddingEventKind::SendingRequest {
                attempt,
                max_attempts,
            } => ChunkStatus::Sending {
                attempt,
                max_attempts,
            },
            EmbeddingEventKind::RateLimit {
                attempt,
                max_attempts,
            } => ChunkStatus::RateLimited {
                attempt,
                max_attempts,
            },
            EmbeddingEventKind::ParsingResponse => ChunkStatus::Parsing,
            EmbeddingEventKind::ProcessingEmbeddings => ChunkStatus::Processing,
            EmbeddingEventKind::Complete { total_elapsed_ms } => ChunkStatus::Completed {
                elapsed_ms: total_elapsed_ms,
            },
            EmbeddingEventKind::Error { message } => ChunkStatus::Failed { reason: message },
            EmbeddingEventKind::MaxRetriesExceeded => ChunkStatus::Failed {
                reason: "max retries exceeded".into(),
            },
            EmbeddingEventKind::RequestSuccessful { .. } => ChunkStatus::Parsing,
            EmbeddingEventKind::ParsingComplete { elapsed_ms } => {
                ChunkStatus::Completed { elapsed_ms }
            }
        }
    }
}

impl std::fmt::Display for ChunkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkStatus::Preparing { count } => {
                write!(f, "Preparing {count} embeddings")
            }
            ChunkStatus::Sending {
                attempt,
                max_attempts,
            } => {
                write!(f, "Sending request ({}/{})", attempt, max_attempts)
            }
            ChunkStatus::RateLimited {
                attempt,
                max_attempts,
            } => {
                write!(
                    f,
                    "{}",
                    format!("Rate limited ({}/{})", attempt, max_attempts).yellow()
                )
            }
            ChunkStatus::Parsing => write!(f, "Parsing response"),
            ChunkStatus::Processing => write!(f, "Processing embeddings"),
            ChunkStatus::Completed { elapsed_ms } => {
                write!(f, "{}", format!("Completed in {} ms", elapsed_ms).green())
            }
            ChunkStatus::Failed { reason } => {
                write!(f, "{}", format!("Failed: {}", reason).red().bold())
            }
        }
    }
}

fn route_embedding_events(
    rx: Receiver<EmbeddingEvent>,
    ui_tx: Sender<EmbeddingEvent>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            ui_tx.send(event).ok();
        }
    })
}

fn log_ui(rx: Receiver<EmbeddingEvent>, max_lines: usize) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut states: BTreeMap<usize, ChunkStatus> = BTreeMap::new();
        let mut order: VecDeque<usize> = VecDeque::new();

        for _ in 0..max_lines {
            eprintln!();
        }

        while let Ok(event) = rx.recv() {
            let job = event.proc_id;
            let new_status = ChunkStatus::from(event);

            if !states.contains_key(&job) {
                order.push_back(job);
            }

            let should_redraw = match states.get(&job) {
                Some(old) => old != &new_status,
                None => true,
            };

            if should_redraw {
                states.insert(job, new_status);

                while order.len() > max_lines {
                    if let Some(old) = order.pop_front() {
                        states.remove(&old);
                    }
                }

                render(&states, &order, max_lines);
            }
        }
    })
}

fn render(states: &BTreeMap<usize, ChunkStatus>, order: &VecDeque<usize>, max_lines: usize) {
    eprint!("\x1b[{}A", max_lines);

    for job in order {
        eprint!("\x1b[2K");
        eprintln!("  [{job:02}] {}", states[job]);
    }

    for _ in order.len()..max_lines {
        eprint!("\x1b[2K");
        eprintln!();
    }

    std::io::stderr().flush().ok();
}
