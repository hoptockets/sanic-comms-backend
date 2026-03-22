use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;
use once_cell::sync::Lazy;
use serde::Serialize;

#[derive(Clone, Serialize, JsonSchema)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub actor_id: String,
    pub action: String,
    pub target: String,
    pub metadata: HashMap<String, String>,
}

static AUDIT_LOG: Lazy<Mutex<Vec<AuditEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn record(actor_id: String, action: String, target: String, metadata: HashMap<String, String>) {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        log.push(AuditEntry {
            id: ulid::Ulid::new().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            actor_id,
            action,
            target,
            metadata,
        });

        if log.len() > 500 {
            let drain_count = log.len().saturating_sub(500);
            log.drain(0..drain_count);
        }
    }
}

pub fn list() -> Vec<AuditEntry> {
    AUDIT_LOG.lock().map(|entries| entries.clone()).unwrap_or_default()
}
