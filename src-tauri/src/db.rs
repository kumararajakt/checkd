//! SQLite persistence for lists and tasks.
//!
//! The models here mirror `src/types.ts` — keep both sides in sync when the
//! data model changes.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// One task, stored in the `tasks` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    /// `YYYY-MM-DD` or null.
    pub due_date: Option<String>,
    /// `HH:MM` (24h) or null.
    pub due_time: Option<String>,
    pub repeat: Option<Repeat>,
}

/// One list, with its tasks nested in display order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub id: String,
    pub name: String,
    pub color: String,
    pub tasks: Vec<Task>,
}

/// Everything the UI needs to render the app.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub lists: Vec<List>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RepeatEnd {
    Never,
    On { date: String },
    After { count: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonthWeek {
    /// 1..=4 — the nth weekday of the month.
    Index(i64),
    /// The last such weekday of the month.
    Last,
}

/// The TS model stores the week as a number or the string `"last"`, so this
/// needs a hand-written serde impl (an untagged unit variant would serialize
/// as `null`).
impl Serialize for MonthWeek {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MonthWeek::Index(n) => serializer.serialize_i64(*n),
            MonthWeek::Last => serializer.serialize_str("last"),
        }
    }
}

impl<'de> Deserialize<'de> for MonthWeek {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(MonthWeek::Index)
                .ok_or_else(|| serde::de::Error::custom("week must be a whole number")),
            serde_json::Value::String(s) if s == "last" => Ok(MonthWeek::Last),
            other => Err(serde::de::Error::custom(format!(
                "unexpected week value: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MonthRepeat {
    /// A day of the month, 1..=31.
    Day { day: i64 },
    /// The last day of the month.
    LastDay,
    /// The nth (or last) weekday of the month.
    Weekday { weekday: i64, week: MonthWeek },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repeat {
    pub unit: RepeatUnit,
    pub interval: i64,
    /// First occurrence, `YYYY-MM-DD`.
    pub start: String,
    /// Time of day for occurrences, `HH:MM` (24h), or null.
    pub time: Option<String>,
    /// Weekdays for weekly rules (0 = Sunday … 6 = Saturday).
    pub weekdays: Vec<i64>,
    /// Anchor within the month; only used by monthly rules.
    pub month_repeat: Option<MonthRepeat>,
    pub end: RepeatEnd,
}

/// Handle to the app's SQLite database, shared by every command.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Opens (creating if needed) the database in the platform app-data dir.
    pub fn open(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Self::open_path(&dir.join("checkd.db"))
    }

    /// Opens the database at an explicit path.
    pub fn open_path(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| e.to_string())?;
        init_schema(&conn)?;

        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        // A poisoned lock means a previous command panicked; the data is still
        // consistent (each statement is its own transaction), so recover.
        self.conn.lock().unwrap_or_else(|err| err.into_inner())
    }

    /// Reads the whole database: lists in order, each with its tasks.
    pub fn load(&self) -> Result<AppData, String> {
        let conn = self.conn();

        let mut stmt = conn
            .prepare("SELECT id, name, color FROM lists ORDER BY position")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut lists = Vec::new();
        for row in rows {
            let (id, name, color) = row.map_err(|e| e.to_string())?;
            let tasks = load_tasks(&conn, &id)?;
            lists.push(List {
                id,
                name,
                color,
                tasks,
            });
        }

        Ok(AppData { lists })
    }

    pub fn create_list(&self, id: &str, name: &str, color: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO lists (id, name, color, position)
                 VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(position), 0) + 1 FROM lists))",
                params![id, name, color],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn rename_list(&self, id: &str, name: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE lists SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn delete_list(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM lists WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_list_color(&self, id: &str, color: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE lists SET color = ?2 WHERE id = ?1",
                params![id, color],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn create_task(&self, id: &str, list_id: &str, title: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json, position)
                 VALUES (?1, ?2, ?3, '', NULL, NULL, NULL,
                         (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE list_id = ?2))",
                params![id, list_id, title],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn rename_task(&self, id: &str, title: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET title = ?2 WHERE id = ?1",
                params![id, title],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_task_due(
        &self,
        id: &str,
        due_date: Option<&str>,
        due_time: Option<&str>,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET due_date = ?2, due_time = ?3 WHERE id = ?1",
                params![id, due_date, due_time],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_task_description(&self, id: &str, description: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET description = ?2 WHERE id = ?1",
                params![id, description],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn set_task_repeat(&self, id: &str, repeat: Option<&Repeat>) -> Result<(), String> {
        let encoded = match repeat {
            Some(rule) => Some(serde_json::to_string(rule).map_err(|e| e.to_string())?),
            None => None,
        };
        self.conn()
            .execute(
                "UPDATE tasks SET repeat_json = ?2 WHERE id = ?1",
                params![id, encoded],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

fn load_tasks(conn: &Connection, list_id: &str) -> Result<Vec<Task>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, due_date, due_time, repeat_json
             FROM tasks WHERE list_id = ?1 ORDER BY position",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![list_id], |row| {
            let repeat_json: Option<String> = row.get(5)?;
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                due_date: row.get(3)?,
                due_time: row.get(4)?,
                repeat: repeat_json.and_then(|json| serde_json::from_str(&json).ok()),
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<Task>, _>>().map_err(|e| e.to_string())
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS lists (
             id       TEXT PRIMARY KEY,
             name     TEXT NOT NULL,
             color    TEXT NOT NULL,
             position INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS tasks (
             id          TEXT PRIMARY KEY,
             list_id     TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
             title       TEXT NOT NULL,
             description TEXT NOT NULL DEFAULT '',
             due_date    TEXT,
             due_time    TEXT,
             repeat_json TEXT,
             position    INTEGER NOT NULL
         );

         CREATE INDEX IF NOT EXISTS idx_tasks_list_id ON tasks(list_id);",
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> Db {
        let path = std::env::temp_dir().join(format!("checkd-test-{name}.db"));
        let _ = std::fs::remove_file(&path);
        Db::open_path(&path).expect("open temp db")
    }

    #[test]
    fn repeat_round_trips_through_the_ts_shape() {
        let json = r#"{
            "unit": "week",
            "interval": 2,
            "start": "2026-01-05",
            "time": "09:30",
            "weekdays": [1, 3],
            "monthRepeat": null,
            "end": { "type": "after", "count": 5 }
        }"#;

        let rule: Repeat = serde_json::from_str(json).unwrap();
        assert_eq!(rule.unit, RepeatUnit::Week);
        assert_eq!(rule.weekdays, vec![1, 3]);
        assert!(matches!(rule.end, RepeatEnd::After { count: 5 }));

        // Serializing must give back exactly what the TS model produced.
        let back = serde_json::to_value(&rule).unwrap();
        assert_eq!(back, serde_json::from_str::<serde_json::Value>(json).unwrap());
    }

    #[test]
    fn month_weekday_keeps_a_number_or_last() {
        let numeric: MonthRepeat =
            serde_json::from_str(r#"{"type":"weekday","weekday":2,"week":3}"#).unwrap();
        let last: MonthRepeat =
            serde_json::from_str(r#"{"type":"weekday","weekday":2,"week":"last"}"#).unwrap();

        assert!(matches!(
            numeric,
            MonthRepeat::Weekday {
                week: MonthWeek::Index(3),
                ..
            }
        ));
        assert!(matches!(
            last,
            MonthRepeat::Weekday {
                week: MonthWeek::Last,
                ..
            }
        ));

        assert_eq!(
            serde_json::to_value(&last).unwrap(),
            serde_json::json!({"type": "weekday", "weekday": 2, "week": "last"})
        );
    }

    #[test]
    fn lists_and_tasks_persist_and_cascade() {
        let db = temp_db("crud");

        db.create_list("l1", "Groceries", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();
        db.set_task_due("t1", Some("2026-10-21"), Some("18:00")).unwrap();

        let data = db.load().unwrap();
        assert_eq!(data.lists.len(), 1);
        assert_eq!(data.lists[0].name, "Groceries");
        assert_eq!(data.lists[0].tasks.len(), 1);
        assert_eq!(data.lists[0].tasks[0].title, "Milk");
        assert_eq!(
            data.lists[0].tasks[0].due_date.as_deref(),
            Some("2026-10-21")
        );
        assert_eq!(data.lists[0].tasks[0].due_time.as_deref(), Some("18:00"));

        // Deleting a list takes its tasks with it.
        db.delete_list("l1").unwrap();
        assert!(db.load().unwrap().lists.is_empty());
    }

    #[test]
    fn task_order_follows_creation_order() {
        let db = temp_db("order");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "first").unwrap();
        db.create_task("t2", "l1", "second").unwrap();

        let data = db.load().unwrap();
        let tasks = &data.lists[0].tasks;
        assert_eq!(tasks[0].title, "first");
        assert_eq!(tasks[1].title, "second");
    }
}
