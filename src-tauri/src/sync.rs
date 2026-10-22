//! Google Calendar sync — the local bookkeeping.
//!
//! The Google side of the sync is not implemented yet; see
//! `docs/google-calendar-sync-design.md` for the full design. What lives
//! here is the part every local write has to take part in: the `sync_links`
//! table, which is at the same time
//!
//! * the **outbox** — one row per local entity, `dirty = 1` while it has
//!   changes that still need to be pushed to Google,
//! * the **id mapping** — the Google calendar id / event id the local
//!   entity belongs to (`remote_id`, `None` until it exists remotely), and
//! * the **tombstone log** — a deleted entity keeps its row with
//!   `deleted = 1` so the remote object can be removed and so a pull cannot
//!   resurrect it.
//!
//! The rows are written by the normal mutation paths in `db.rs`, so the
//! outbox is a side effect of using the app — there is no separate "sync
//! this" step the user could forget.

/// What a sync link points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEntity {
    /// A list — mirrored as a Google Calendar.
    List,
    /// A task — mirrored as a Google Event.
    Task,
}

impl SyncEntity {
    /// The value stored in `sync_links.entity_type`.
    pub fn as_str(self) -> &'static str {
        match self {
            SyncEntity::List => "list",
            SyncEntity::Task => "task",
        }
    }

    /// Parses a stored `entity_type`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "list" => Some(SyncEntity::List),
            "task" => Some(SyncEntity::Task),
            _ => None,
        }
    }
}

/// One row of the outbox.
#[derive(Debug, Clone)]
pub struct SyncLink {
    pub entity: SyncEntity,
    /// The local id — the list id or the task id.
    pub entity_id: String,
    /// The Google calendar id / event id. `None` until it exists remotely.
    pub remote_id: Option<String>,
    /// The local change still has to be pushed.
    pub dirty: bool,
    /// The entity was deleted locally; the remote object still has to go.
    pub deleted: bool,
    /// When the local change happened (unix milliseconds).
    pub updated_at: i64,
    /// When the last push of this entity succeeded (unix milliseconds).
    pub synced_at: i64,
}

/// Key used in `sync_state` for "the user connected a Google account".
pub const STATE_ENABLED: &str = "enabled";
/// Key used in `sync_state` for the connected Google account id.
pub const STATE_ACCOUNT: &str = "account";
/// Key used in `sync_state` for the time of the last successful sync.
pub const STATE_SYNCED_AT: &str = "synced_at";
