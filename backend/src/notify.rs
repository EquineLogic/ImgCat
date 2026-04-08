use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event")]
pub enum SharingEvent {
    NewShareRequest {
        request_id: Uuid,
        filesystem_id: Uuid,
        entry_name: String,
        sender_username: String,
        access_level: String,
    },
    ShareRequestAccepted {
        request_id: Uuid,
        filesystem_id: Uuid,
        recipient_username: String,
    },
    ShareRequestDeclined {
        request_id: Uuid,
        recipient_username: String,
    },
    ShareRequestCancelled {
        request_id: Uuid,
    },
    PermissionRevoked {
        permission_id: Uuid,
        filesystem_id: Uuid,
    },
}

pub struct NotifyHub {
    channels: Mutex<HashMap<Uuid, broadcast::Sender<SharingEvent>>>,
}

impl NotifyHub {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a user's notification channel. Creates the channel lazily.
    pub fn subscribe(&self, user_id: Uuid) -> broadcast::Receiver<SharingEvent> {
        let mut map = self.channels.lock().unwrap();
        let tx = map.entry(user_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(64);
            tx
        });
        tx.subscribe()
    }

    /// Send an event to a specific user. No-op if nobody is listening.
    pub fn send_to(&self, user_id: Uuid, event: SharingEvent) {
        let map = self.channels.lock().unwrap();
        if let Some(tx) = map.get(&user_id) {
            let _ = tx.send(event);
        }
    }

    /// Remove a user's channel when their last WS disconnects.
    pub fn remove_if_empty(&self, user_id: Uuid) {
        let mut map = self.channels.lock().unwrap();
        if let Some(tx) = map.get(&user_id) {
            if tx.receiver_count() == 0 {
                map.remove(&user_id);
            }
        }
    }
}
