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
}
