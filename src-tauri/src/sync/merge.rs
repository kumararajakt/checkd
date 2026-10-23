use crate::db::{SyncList, SyncTask};
use crate::sync::payload::{ListPayload, TaskPayload};
#[derive(Debug, Clone, PartialEq)]
pub enum ListAction {
    Create { list: SyncList, payload: ListPayload },
    Update { list: SyncList, payload: ListPayload },
    Ignore,
    Tombstone,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TaskAction {
    Create { task: SyncTask, payload: TaskPayload },
    Update { task: SyncTask, payload: TaskPayload },
    Ignore,
    Tombstone,
}
#[derive(Debug, Clone)]
pub struct LocalState {
    pub exists: bool,
    pub updated_at: Option<i64>,
    pub dirty: bool,
    pub deleted: bool,
}
pub fn merge_list(
    remote_payload: &ListPayload,
    local: Option<&LocalState>,
    link_deleted: bool,
    link_dirty: bool,
) -> ListAction {
    let state = local.cloned().unwrap_or(LocalState {
        exists: false,
        updated_at: None,
        dirty: link_dirty,
        deleted: link_deleted,
    });
    if state.deleted || link_deleted {
        return ListAction::Tombstone;
    }
    if state.dirty {
        if is_remote_newer(remote_payload.updated_at, state.updated_at) {
            return ListAction::Update {
                list: payload_to_list(remote_payload),
                payload: remote_payload.clone(),
            };
        }
        return ListAction::Ignore;
    }
    if state.exists {
        if is_remote_newer(remote_payload.updated_at, state.updated_at) {
            return ListAction::Update {
                list: payload_to_list(remote_payload),
                payload: remote_payload.clone(),
            };
        }
        return ListAction::Ignore;
    }
    ListAction::Create {
        list: payload_to_list(remote_payload),
        payload: remote_payload.clone(),
    }
}
pub fn merge_task(
    remote_payload: &TaskPayload,
    local: Option<&LocalState>,
    link_deleted: bool,
    link_dirty: bool,
) -> TaskAction {
    let state = local.cloned().unwrap_or(LocalState {
        exists: false,
        updated_at: None,
        dirty: link_dirty,
        deleted: link_deleted,
    });
    if state.deleted || link_deleted {
        return TaskAction::Tombstone;
    }
    if state.dirty {
        if is_remote_newer(remote_payload.updated_at, state.updated_at) {
            return TaskAction::Update {
                task: payload_to_task(remote_payload),
                payload: remote_payload.clone(),
            };
        }
        return TaskAction::Ignore;
    }
    if state.exists {
        if is_remote_newer(remote_payload.updated_at, state.updated_at) {
            return TaskAction::Update {
                task: payload_to_task(remote_payload),
                payload: remote_payload.clone(),
            };
        }
        return TaskAction::Ignore;
    }
    TaskAction::Create {
        task: payload_to_task(remote_payload),
        payload: remote_payload.clone(),
    }
}
fn is_remote_newer(remote: i64, local: Option<i64>) -> bool {
    match local {
        Some(local) => remote > local,
        None => true,
    }
}
pub fn payload_to_list(payload: &ListPayload) -> SyncList {
    SyncList {
        id: payload.id.clone(),
        name: payload.name.clone(),
        color: payload.color.clone(),
        position: payload.position,
        updated_at: payload.updated_at,
    }
}
pub fn payload_to_task(payload: &TaskPayload) -> SyncTask {
    SyncTask {
        id: payload.id.clone(),
        list_id: payload.list_id.clone(),
        title: payload.title.clone(),
        description: payload.description.clone(),
        due_date: payload.due_date.clone(),
        due_time: payload.due_time.clone(),
        repeat: payload.repeat.clone(),
        completed: payload.completed,
        next_occurrence: payload.next_occurrence.clone(),
        occurrence_index: payload.occurrence_index,
        position: payload.position,
        updated_at: payload.updated_at,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalAbsent {
    Keep,
    Delete,
}
pub fn handle_local_absent(dirty: bool) -> LocalAbsent {
    if dirty { LocalAbsent::Keep } else { LocalAbsent::Delete }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::payload::{KIND_LIST, KIND_TASK, PAYLOAD_VERSION};
    fn list_payload() -> ListPayload {
        ListPayload {
            version: PAYLOAD_VERSION,
            kind: KIND_LIST.to_string(),
            id: "list-1".to_string(),
            name: "Groceries".to_string(),
            color: "#3b82f6".to_string(),
            position: 1,
            updated_at: 2000,
        }
    }
    fn task_payload() -> TaskPayload {
        TaskPayload {
            version: PAYLOAD_VERSION,
            kind: KIND_TASK.to_string(),
            id: "task-1".to_string(),
            list_id: "list-1".to_string(),
            title: "Milk".to_string(),
            description: "2%".to_string(),
            due_date: None,
            due_time: None,
            repeat: None,
            completed: false,
            next_occurrence: None,
            occurrence_index: 0,
            position: 0,
            updated_at: 2000,
            truncated: false,
        }
    }
    #[test]
    fn clean_local_absent_remote_gets_created() {
        let action = merge_list(&list_payload(), None, false, false);
        assert!(matches!(action, ListAction::Create { .. }));
    }
    #[test]
    fn clean_local_newer_remote_is_ignored() {
        let local = LocalState {
            exists: true,
            updated_at: Some(3000),
            dirty: false,
            deleted: false,
        };
        let action = merge_list(&list_payload(), Some(&local), false, false);
        assert!(matches!(action, ListAction::Ignore));
    }
    #[test]
    fn clean_local_older_remote_is_applied() {
        let local = LocalState {
            exists: true,
            updated_at: Some(1000),
            dirty: false,
            deleted: false,
        };
        let action = merge_list(&list_payload(), Some(&local), false, false);
        assert!(matches!(action, ListAction::Update { .. }));
    }
    #[test]
    fn dirty_local_older_remote_is_applied() {
        let local = LocalState {
            exists: true,
            updated_at: Some(1000),
            dirty: true,
            deleted: false,
        };
        let action = merge_list(&list_payload(), Some(&local), false, true);
        assert!(matches!(action, ListAction::Update { .. }));
    }
    #[test]
    fn dirty_local_newer_remote_is_ignored_so_push_wins() {
        let local = LocalState {
            exists: true,
            updated_at: Some(3000),
            dirty: true,
            deleted: false,
        };
        let action = merge_list(&list_payload(), Some(&local), false, true);
        assert!(matches!(action, ListAction::Ignore));
    }
    #[test]
    fn tombstoned_local_keeps_the_tombstone() {
        let action = merge_list(&list_payload(), None, true, true);
        assert!(matches!(action, ListAction::Tombstone));
    }
    #[test]
    fn dirty_local_absent_remote_is_kept_for_push() {
        assert!(matches!(handle_local_absent(true), LocalAbsent::Keep));
    }
    #[test]
    fn clean_local_absent_remote_is_deleted_locally() {
        assert!(matches!(handle_local_absent(false), LocalAbsent::Delete));
    }
    #[test]
    fn task_merge_follows_the_same_rules() {
        let local = LocalState {
            exists: true,
            updated_at: Some(1000),
            dirty: false,
            deleted: false,
        };
        let action = merge_task(&task_payload(), Some(&local), false, false);
        assert!(matches!(action, TaskAction::Update { .. }));
        let local = LocalState {
            exists: true,
            updated_at: Some(3000),
            dirty: false,
            deleted: false,
        };
        let action = merge_task(&task_payload(), Some(&local), false, false);
        assert!(matches!(action, TaskAction::Ignore));
    }
}
