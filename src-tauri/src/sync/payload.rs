use serde::{Deserialize, Serialize};
use crate::db::{Repeat, SyncList, SyncTask};
use crate::sync::google::NAMESPACE;
pub const PAYLOAD_VERSION: u32 = 1;
pub const DUMMY_DATE: &str = "2022-06-27";
const MAX_PAYLOAD: usize = 8000;
pub const KIND_LIST: &str = "list";
pub const KIND_TASK: &str = "task";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPayload {
    pub version: u32,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPayload {
    pub version: u32,
    pub kind: String,
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub repeat: Option<Repeat>,
    pub completed: bool,
    pub next_occurrence: Option<String>,
    pub occurrence_index: i64,
    pub position: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub truncated: bool,
}
impl TaskPayload {
    pub fn parse(json: &str) -> Option<TaskPayload> {
        let payload: TaskPayload = serde_json::from_str(json).ok()?;
        if payload.kind != KIND_TASK || payload.version > PAYLOAD_VERSION {
            return None;
        }
        Some(payload)
    }
}
impl ListPayload {
    pub fn parse(json: &str) -> Option<ListPayload> {
        let payload: ListPayload = serde_json::from_str(json).ok()?;
        if payload.kind != KIND_LIST || payload.version > PAYLOAD_VERSION {
            return None;
        }
        Some(payload)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDate {
    pub date: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    #[serde(rename = "iCalUID")]
    pub ical_uid: String,
    pub summary: String,
    pub description: String,
    pub status: String,
    pub start: EventDate,
    pub end: EventDate,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarPayload {
    pub summary: String,
    pub description: String,
    pub location: String,
}
pub fn task_event(task: &SyncTask) -> EventPayload {
    EventPayload {
        ical_uid: task.id.clone(),
        summary: task.title.clone(),
        description: task_payload_json(task),
        status: "confirmed".to_string(),
        start: EventDate {
            date: DUMMY_DATE.to_string(),
        },
        end: EventDate {
            date: DUMMY_DATE.to_string(),
        },
    }
}
pub fn list_calendar(list: &SyncList) -> CalendarPayload {
    CalendarPayload {
        summary: list.name.clone(),
        description: list_payload_json(list),
        location: NAMESPACE.to_string(),
    }
}
pub fn task_payload_json(task: &SyncTask) -> String {
    let mut payload = TaskPayload {
        version: PAYLOAD_VERSION,
        kind: KIND_TASK.to_string(),
        id: task.id.clone(),
        list_id: task.list_id.clone(),
        title: task.title.clone(),
        description: task.description.clone(),
        due_date: task.due_date.clone(),
        due_time: task.due_time.clone(),
        repeat: task.repeat.clone(),
        completed: task.completed,
        next_occurrence: task.next_occurrence.clone(),
        occurrence_index: task.occurrence_index,
        position: task.position,
        updated_at: task.updated_at,
        truncated: false,
    };
    let json = encode(&payload);
    if json.len() <= MAX_PAYLOAD {
        return json;
    }
    shorten(&mut payload);
    encode(&payload)
}
pub fn list_payload_json(list: &SyncList) -> String {
    let payload = ListPayload {
        version: PAYLOAD_VERSION,
        kind: KIND_LIST.to_string(),
        id: list.id.clone(),
        name: list.name.clone(),
        color: list.color.clone(),
        position: list.position,
        updated_at: list.updated_at,
    };
    encode(&payload)
}
fn encode<T: Serialize>(payload: &T) -> String {
    serde_json::to_string(payload).unwrap_or_default()
}
fn shorten(payload: &mut TaskPayload) {
    payload.truncated = true;
    let mut description: Vec<char> = payload.description.chars().collect();
    while !description.is_empty() {
        payload.description = description.iter().collect();
        if encode(payload).len() <= MAX_PAYLOAD {
            return;
        }
        let cut = (description.len() / 10).max(1);
        description.truncate(description.len() - cut);
    }
    payload.description = String::new();
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{MonthRepeat, MonthWeek, RepeatEnd, RepeatUnit};
    fn task() -> SyncTask {
        SyncTask {
            id: "task-1".to_string(),
            list_id: "list-1".to_string(),
            title: "Buy milk".to_string(),
            description: "The good one".to_string(),
            due_date: Some("2026-10-22".to_string()),
            due_time: Some("18:00".to_string()),
            repeat: None,
            completed: false,
            next_occurrence: None,
            occurrence_index: 0,
            position: 3,
            updated_at: 1_790_000_000_000,
        }
    }
    fn list() -> SyncList {
        SyncList {
            id: "list-1".to_string(),
            name: "Groceries".to_string(),
            color: "#3b82f6".to_string(),
            position: 1,
            updated_at: 1_790_000_000_000,
        }
    }
    #[test]
    fn a_task_becomes_an_event() {
        let event = task_event(&task());
        assert_eq!(event.ical_uid, "task-1");
        assert_eq!(event.summary, "Buy milk");
        assert_eq!(event.status, "confirmed");
        assert_eq!(event.start.date, "2022-06-27");
        assert_eq!(event.end.date, "2022-06-27");
    }
    #[test]
    fn the_payload_travels_as_json_in_the_description() {
        let event = task_event(&task());
        let payload = TaskPayload::parse(&event.description).expect("the payload is there");
        assert_eq!(payload.version, PAYLOAD_VERSION);
        assert_eq!(payload.kind, KIND_TASK);
        assert_eq!(payload.id, "task-1");
        assert_eq!(payload.list_id, "list-1");
        assert_eq!(payload.title, "Buy milk");
        assert_eq!(payload.description, "The good one");
        assert_eq!(payload.due_date.as_deref(), Some("2026-10-22"));
        assert_eq!(payload.due_time.as_deref(), Some("18:00"));
        assert!(!payload.completed);
        assert_eq!(payload.position, 3);
        assert_eq!(payload.updated_at, 1_790_000_000_000);
        assert!(!payload.truncated);
    }
    #[test]
    fn the_repeat_rule_survives_the_round_trip() {
        let mut task = task();
        task.repeat = Some(Repeat {
            unit: RepeatUnit::Month,
            interval: 2,
            start: "2026-10-22".to_string(),
            time: Some("09:00".to_string()),
            weekdays: vec![],
            month_repeat: Some(MonthRepeat::Weekday {
                weekday: 3,
                week: MonthWeek::Last,
            }),
            end: RepeatEnd::After { count: 5 },
        });
        let event = task_event(&task);
        let payload = TaskPayload::parse(&event.description).expect("the payload is there");
        let rule = payload.repeat.expect("the rule is there");
        assert_eq!(rule.unit, RepeatUnit::Month);
        assert_eq!(rule.interval, 2);
        assert_eq!(rule.start, "2026-10-22");
        assert_eq!(rule.time.as_deref(), Some("09:00"));
        assert!(matches!(
            rule.month_repeat,
            Some(MonthRepeat::Weekday {
                week: MonthWeek::Last,
                ..
            })
        ));
        assert!(matches!(rule.end, RepeatEnd::After { count: 5 }));
    }
    #[test]
    fn every_repeat_end_survives_the_round_trip() {
        for end in [
            RepeatEnd::Never,
            RepeatEnd::On {
                date: "2027-01-01".to_string(),
            },
            RepeatEnd::After { count: 3 },
        ] {
            let mut task = task();
            task.repeat = Some(Repeat {
                unit: RepeatUnit::Week,
                interval: 1,
                start: "2026-10-22".to_string(),
                time: None,
                weekdays: vec![1, 3],
                month_repeat: None,
                end: end.clone(),
            });
            let event = task_event(&task);
            let payload = TaskPayload::parse(&event.description).expect("the payload is there");
            assert_eq!(
                serde_json::to_string(&payload.repeat.unwrap().end).unwrap(),
                serde_json::to_string(&end).unwrap()
            );
        }
    }
    #[test]
    fn a_long_description_is_shortened() {
        let mut task = task();
        task.description = "milk ".repeat(5000);
        let event = task_event(&task);
        assert!(event.description.len() <= MAX_PAYLOAD);
        let payload = TaskPayload::parse(&event.description).expect("the payload is there");
        assert!(payload.truncated);
        assert!(payload.description.len() < task.description.len());
        assert_eq!(payload.title, "Buy milk");
        assert_eq!(payload.due_date.as_deref(), Some("2026-10-22"));
    }
    #[test]
    fn a_list_becomes_a_calendar_in_its_own_namespace() {
        let calendar = list_calendar(&list());
        assert_eq!(calendar.summary, "Groceries");
        assert_eq!(calendar.location, "checkd");
        let payload = ListPayload::parse(&calendar.description).expect("the payload is there");
        assert_eq!(payload.id, "list-1");
        assert_eq!(payload.name, "Groceries");
        assert_eq!(payload.color, "#3b82f6");
        assert_eq!(payload.position, 1);
    }
    #[test]
    fn a_description_that_is_not_a_payload_is_refused() {
        assert!(TaskPayload::parse("Bring milk home").is_none());
        assert!(TaskPayload::parse("{}").is_none());
        assert!(TaskPayload::parse("{ broken").is_none());
        let newer = r#"{"version":99,"kind":"task","id":"t","listId":"l","title":"x",
            "description":"","dueDate":null,"dueTime":null,"repeat":null,"completed":false,
            "nextOccurrence":null,"occurrenceIndex":0,"position":0,"updatedAt":0}"#;
        assert!(TaskPayload::parse(newer).is_none());
    }
}
