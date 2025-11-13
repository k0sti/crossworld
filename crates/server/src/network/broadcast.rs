use crate::protocol::WorldUpdate;
use tokio::sync::broadcast;

/// Lightweight wrapper around Tokio broadcast channels so other modules do not
/// have to manage the plumbing directly.
#[derive(Clone)]
pub struct BroadcastHub {
    sender: broadcast::Sender<WorldUpdate>,
}

impl BroadcastHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WorldUpdate> {
        self.sender.subscribe()
    }

    pub fn publish(&self, update: WorldUpdate) {
        let _ = self.sender.send(update);
    }
}
