use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};

use tracing::Level;

/// A single captured log entry.
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: Level,
    pub message: String,
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:>5} {}", self.timestamp, self.level, self.message)
    }
}

/// Shared in-memory ring buffer for captured log entries.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<LogEntry>>>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    /// Push a new log entry, evicting the oldest if at capacity.
    pub fn push(&self, entry: LogEntry) {
        let mut buf = self.inner.lock().unwrap();
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Get the most recent `n` entries (oldest first).
    pub fn recent(&self, n: usize) -> Vec<LogEntry> {
        let buf = self.inner.lock().unwrap();
        let skip = buf.len().saturating_sub(n);
        buf.iter().skip(skip).cloned().collect()
    }

    /// Total number of entries currently stored.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Get all entries as a snapshot.
    pub fn all(&self) -> Vec<LogEntry> {
        self.inner.lock().unwrap().iter().cloned().collect()
    }
}

/// A `tracing_subscriber::Layer` that captures log events into a `LogBuffer`.
pub struct LogBufferLayer {
    buffer: LogBuffer,
}

impl LogBufferLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> tracing_subscriber::Layer<S> for LogBufferLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Extract the message from the event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let now = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

        let entry = LogEntry {
            timestamp: now,
            level: *event.metadata().level(),
            message: visitor
                .message
                .unwrap_or_else(|| format!("{:?}", event.metadata().name())),
        };

        self.buffer.push(entry);
    }
}

/// Visitor that extracts the `message` field from a tracing event.
#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else if self.message.is_none() {
            // Fallback: use the first field as message
            self.message = Some(format!("{}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}
