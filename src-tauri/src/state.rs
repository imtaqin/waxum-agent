use tokio::process::Child;
use tokio::sync::{watch, Mutex};

/// Process-wide state: the bundled-waxum child process (if running) and the
/// stop signal for the current event-stream task. Both are `None` when
/// idle.
#[derive(Default)]
pub struct AppState {
    pub bundled_child: Mutex<Option<Child>>,
    pub event_stop: Mutex<Option<watch::Sender<bool>>>,
    pub pairing_stop: Mutex<Option<watch::Sender<bool>>>,
    /// Rolling chat-completion message history for `ai::converse` -- kept
    /// process-wide (single user, single ongoing conversation) so a
    /// follow-up like "kamu balas apa barusan?" has something to answer
    /// from instead of a fresh, stateless request every turn.
    pub ai_history: Mutex<Vec<serde_json::Value>>,
}
