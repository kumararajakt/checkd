//! SQLite persistence for lists and tasks.
//!
//! The models here mirror `src/types.ts` — keep both sides in sync when the
//! data model changes.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::repeat;
use crate::sync::{SyncEntity, SyncLink};

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
    /// Whether the active occurrence is done.
    pub completed: bool,
    /// Date of the active occurrence (repeating tasks only).
    pub next_occurrence: Option<String>,
    /// Index of the active occurrence within the series.
    pub occurrence_index: i64,
    /// Key of the notification already sent for the active occurrence.
    /// Internal — never sent to the frontend.
    #[serde(skip_serializing, default)]
    pub notified_key: Option<String>,
}

impl Task {
    /// The moment this task is due, plus a key identifying that occurrence.
    ///
    /// Repeating tasks are due at their active occurrence; one-off tasks at
    /// their due date. Tasks without a due time are due at 09:00 on the due
    /// day.
    pub fn due_moment(&self) -> Option<(NaiveDateTime, String)> {
        if self.completed {
            return None;
        }

        let (date, time, key) = match (&self.repeat, &self.next_occurrence) {
            (Some(rule), Some(occurrence)) => {
                let date = repeat::parse_date(occurrence)?;
                let time = parse_time(rule.time.as_deref());
                (date, time, format!("occ:{occurrence}"))
            }
            _ => {
                let date = repeat::parse_date(self.due_date.as_deref()?)?;
                let time = parse_time(self.due_time.as_deref());
                let key = match &self.due_time {
                    Some(time) => format!("due:{}T{}", self.due_date.as_deref()?, time),
                    None => format!("due:{}", self.due_date.as_deref()?),
                };
                (date, time, key)
            }
        };

        Some((date.and_time(time), key))
    }
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

/// A task together with the list it belongs to (used by the notifier).
#[derive(Debug, Clone)]
pub struct TaskWithList {
    pub task: Task,
    pub list_name: String,
}

/// A list row with its position and `updated_at` timestamp (used by the sync engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncList {
    pub id: String,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub updated_at: i64,
}

/// A task row with its `list_id`, `position`, and `updated_at` timestamp (used by the sync engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTask {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MonthRepeat {
    /// A day of the month, 1..=31.
    Day { day: i64 },
    /// The last day of the month.
    LastDay,
    /// The nth (or last) weekday of the month.
    Weekday { weekday: i64, week: MonthWeek },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    /// All tasks with their list name — the notifier's work list.
    pub fn tasks_for_notifications(&self) -> Result<Vec<TaskWithList>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT t.id, t.title, t.description, t.due_date, t.due_time, t.repeat_json,
                        t.completed, t.next_occurrence, t.occurrence_index, t.notified_key,
                        l.name
                 FROM tasks t JOIN lists l ON l.id = t.list_id",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let repeat_json: Option<String> = row.get(5)?;
                Ok(TaskWithList {
                    task: Task {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        description: row.get(2)?,
                        due_date: row.get(3)?,
                        due_time: row.get(4)?,
                        repeat: repeat_json.and_then(|json| serde_json::from_str(&json).ok()),
                        completed: row.get::<_, i64>(6)? != 0,
                        next_occurrence: row.get(7)?,
                        occurrence_index: row.get(8)?,
                        notified_key: row.get(9)?,
                    },
                    list_name: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Advances repeating tasks whose occurrence was missed.
    ///
    /// The pointer normally moves on when an occurrence is completed. When it
    /// is not, the series would stall — so once the *next* occurrence is due,
    /// the missed one is dropped and the pointer moves on. That keeps the
    /// reminders coming (like a habit tracker) instead of going silent after
    /// a single missed day.
    ///
    /// Returns whether any task moved on.
    pub fn roll_forward_missed(&self, now: NaiveDateTime) -> Result<bool, String> {
        let tasks = self.tasks_for_notifications()?;
        let mut changed = false;

        for entry in tasks {
            let task = &entry.task;
            let Some(rule) = task.repeat.as_ref() else {
                continue;
            };
            let Some(current) = task
                .next_occurrence
                .as_deref()
                .and_then(repeat::parse_date)
            else {
                continue;
            };

            // Walk forward while the following occurrence is already due.
            let mut occurrence = current;
            let mut index = task.occurrence_index;
            while let Some(next) = repeat::next_after(rule, occurrence) {
                if occurrence_moment(next, rule) > now {
                    break;
                }
                occurrence = next;
                index += 1;
            }

            if occurrence == current {
                continue;
            }
            changed = true;

            if series_ended(rule, index) {
                // The series is over: the task becomes a plain completed task.
                self.conn()
                    .execute(
                        "UPDATE tasks
                         SET completed = 1, repeat_json = NULL, next_occurrence = NULL,
                             occurrence_index = ?2, notified_key = NULL
                         WHERE id = ?1",
                        params![task.id, index],
                    )
                    .map_err(|e| e.to_string())?;
                self.touch(SyncEntity::Task, &task.id)?;
            } else {
                self.conn()
                    .execute(
                        "UPDATE tasks
                         SET completed = 0, next_occurrence = ?2, occurrence_index = ?3,
                             notified_key = NULL
                         WHERE id = ?1",
                        params![task.id, repeat::format_date(occurrence), index],
                    )
                    .map_err(|e| e.to_string())?;
                self.touch(SyncEntity::Task, &task.id)?;
            }
        }

        Ok(changed)
    }

    /// Forgets which reminders were already sent.
    ///
    /// Called once when the app starts: every launch is a fresh reminder
    /// session, so whatever is overdue is brought up again — even when it was
    /// already reminded about in an earlier session.
    pub fn reset_notifications(&self) -> Result<(), String> {
        self.conn()
            .execute("UPDATE tasks SET notified_key = NULL", [])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Records that the notification for an occurrence was sent.
    pub fn mark_notified(&self, id: &str, key: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET notified_key = ?2 WHERE id = ?1",
                params![id, key],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Reads a single task.
    pub fn get_task(&self, id: &str) -> Result<Option<Task>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, description, due_date, due_time, repeat_json, completed,
                        next_occurrence, occurrence_index, notified_key
                 FROM tasks WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let repeat_json: Option<String> = row.get(5)?;
                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    due_date: row.get(3)?,
                    due_time: row.get(4)?,
                    repeat: repeat_json.and_then(|json| serde_json::from_str(&json).ok()),
                    completed: row.get::<_, i64>(6)? != 0,
                    next_occurrence: row.get(7)?,
                    occurrence_index: row.get(8)?,
                    notified_key: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn create_list(&self, id: &str, name: &str, color: &str) -> Result<(), String> {
        let now = now_ms();
        self.conn()
            .execute(
                "INSERT INTO lists (id, name, color, position, updated_at)
                 VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(position), 0) + 1 FROM lists), ?4)",
                params![id, name, color, now],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.mark_dirty(SyncEntity::List, id, None)
    }

    pub fn rename_list(&self, id: &str, name: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE lists SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.touch(SyncEntity::List, id)
    }

    pub fn delete_list(&self, id: &str) -> Result<(), String> {
        // The tasks go with the list (ON DELETE CASCADE), so they need a
        // tombstone of their own — a pull must not bring them back.
        let task_ids = self.task_ids_of(id)?;

        self.conn()
            .execute("DELETE FROM lists WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.mark_deleted(SyncEntity::List, id, None)?;
        for task_id in task_ids {
            self.mark_deleted(SyncEntity::Task, &task_id, Some(id))?;
        }

        Ok(())
    }

    pub fn set_list_color(&self, id: &str, color: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE lists SET color = ?2 WHERE id = ?1",
                params![id, color],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.touch(SyncEntity::List, id)
    }

    pub fn create_task(&self, id: &str, list_id: &str, title: &str) -> Result<(), String> {
        let now = now_ms();
        self.conn()
            .execute(
                "INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json, position, updated_at)
                 VALUES (?1, ?2, ?3, '', NULL, NULL, NULL,
                         (SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE list_id = ?2), ?4)",
                params![id, list_id, title, now],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.mark_dirty(SyncEntity::Task, id, Some(list_id))
    }

    pub fn rename_task(&self, id: &str, title: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET title = ?2 WHERE id = ?1",
                params![id, title],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.touch(SyncEntity::Task, id)
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
            .map_err(|e| e.to_string())?;

        self.refresh_notification_state(id)?;
        self.touch(SyncEntity::Task, id)
    }

    pub fn set_task_description(&self, id: &str, description: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks SET description = ?2 WHERE id = ?1",
                params![id, description],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.touch(SyncEntity::Task, id)
    }

    /// Sets (or clears) the repeat rule and re-anchors the active occurrence.
    pub fn set_task_repeat(&self, id: &str, repeat: Option<&Repeat>) -> Result<Task, String> {
        let (encoded, next_occurrence) = match repeat {
            Some(rule) => {
                let encoded = serde_json::to_string(rule).map_err(|e| e.to_string())?;
                let today = chrono::Local::now().date_naive();
                let next = repeat::first_at_or_after(rule, today).map(repeat::format_date);
                (Some(encoded), next)
            }
            None => (None, None),
        };

        self.conn()
            .execute(
                "UPDATE tasks
                 SET repeat_json = ?2, next_occurrence = ?3, occurrence_index = 0, notified_key = NULL
                 WHERE id = ?1",
                params![id, encoded, next_occurrence],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.refresh_notification_state(id)?;
        self.touch(SyncEntity::Task, id)?;
        self.get_task(id)?.ok_or_else(|| format!("no task with id {id}"))
    }

    /// Marks a task done (or not done).
    ///
    /// Completing the active occurrence of a repeating task advances the
    /// series to its next occurrence; when the series has run out, the rule is
    /// cleared and the task stays completed.
    pub fn set_task_completed(&self, id: &str, completed: bool) -> Result<Task, String> {
        let Some(mut task) = self.get_task(id)? else {
            return Err(format!("no task with id {id}"));
        };

        if !completed {
            self.update_completion(
                id,
                false,
                task.next_occurrence.clone(),
                task.occurrence_index,
            )?;
            self.refresh_notification_state(id)?;
            task.completed = false;
            return Ok(task);
        }

        let Some(rule) = task.repeat.clone() else {
            self.update_completion(id, true, None, 0)?;
            self.refresh_notification_state(id)?;
            task.completed = true;
            task.notified_key = None;
            return Ok(task);
        };

        let index = task.occurrence_index + 1;
        let ended = series_ended(&rule, index);

        let current = task
            .next_occurrence
            .as_deref()
            .and_then(repeat::parse_date)
            .or_else(|| repeat::parse_date(rule.start.as_str()));

        let next = if ended {
            None
        } else {
            current.and_then(|current| repeat::next_after(&rule, current))
        };

        if next.is_none() {
            // The series is over: the task becomes a plain completed task.
            self.conn()
                .execute(
                    "UPDATE tasks
                     SET completed = 1, repeat_json = NULL, next_occurrence = NULL,
                         occurrence_index = ?2, notified_key = NULL
                     WHERE id = ?1",
                    params![id, index],
                )
                .map_err(|e| e.to_string())?;
            self.touch(SyncEntity::Task, id)?;
        } else {
            self.conn()
                .execute(
                    "UPDATE tasks
                     SET completed = 0, next_occurrence = ?2, occurrence_index = ?3, notified_key = NULL
                     WHERE id = ?1",
                    params![id, next, index],
                )
                .map_err(|e| e.to_string())?;
            self.touch(SyncEntity::Task, id)?;
        }

        self.refresh_notification_state(id)?;
        self.get_task(id)?.ok_or_else(|| format!("no task with id {id}"))
    }

    /// Decides whether the current occurrence still deserves a reminder.
    ///
    /// An occurrence whose moment has already passed when the user sets it is
    /// marked as handled — the user just saw it, so no notification is sent.
    /// Everything else stays open, so the scheduler reminds the user when the
    /// moment arrives (or once on the next launch, if the app was closed).
    fn refresh_notification_state(&self, id: &str) -> Result<(), String> {
        let key = match self.get_task(id)? {
            Some(task) => match task.due_moment() {
                Some((moment, key)) => {
                    if moment <= chrono::Local::now().naive_local() {
                        Some(key)
                    } else {
                        None
                    }
                }
                None => None,
            },
            None => None,
        };

        self.conn()
            .execute(
                "UPDATE tasks SET notified_key = ?2 WHERE id = ?1",
                params![id, key],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn update_completion(
        &self,
        id: &str,
        completed: bool,
        next_occurrence: Option<String>,
        occurrence_index: i64,
    ) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE tasks
                 SET completed = ?2, next_occurrence = ?3, occurrence_index = ?4, notified_key = NULL
                 WHERE id = ?1",
                params![id, completed as i64, next_occurrence, occurrence_index],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.touch(SyncEntity::Task, id)
    }

    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        let parent_id = self.task_list_id(id)?;

        self.conn()
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        // The row is gone, but the remote object still has to be deleted —
        // the tombstone in `sync_links` keeps that job (and stops a pull
        // from bringing the task back).
        self.mark_deleted(SyncEntity::Task, id, parent_id.as_deref())
    }

    // --- sync bookkeeping ---------------------------------------------------

    /// The list a task belongs to.
    fn task_list_id(&self, id: &str) -> Result<Option<String>, String> {
        self.conn()
            .query_row(
                "SELECT list_id FROM tasks WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })
    }

    /// Reads a list row as a `SyncList`.
    pub fn sync_list(&self, id: &str) -> Result<Option<SyncList>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, name, color, position, updated_at FROM lists WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(SyncList {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    position: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    /// Reads a task row as a `SyncTask`.
    pub fn sync_task(&self, id: &str) -> Result<Option<SyncTask>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, list_id, title, description, due_date, due_time, repeat_json,
                        completed, next_occurrence, occurrence_index, position, updated_at
                 FROM tasks WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                let repeat_json: Option<String> = row.get(6)?;
                Ok(SyncTask {
                    id: row.get(0)?,
                    list_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    due_date: row.get(4)?,
                    due_time: row.get(5)?,
                    repeat: repeat_json.and_then(|json| serde_json::from_str(&json).ok()),
                    completed: row.get::<_, i64>(7)? != 0,
                    next_occurrence: row.get(8)?,
                    occurrence_index: row.get(9)?,
                    position: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    /// Upserts a list pulled from Google and marks its sync link clean.
    pub fn apply_remote_list(&self, list: &SyncList, remote_id: &str) -> Result<(), String> {
        let now = now_ms();
        self.conn()
            .execute(
                "INSERT INTO lists (id, name, color, position, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     color = excluded.color,
                     position = excluded.position,
                     updated_at = excluded.updated_at",
                params![list.id, list.name, list.color, list.position, list.updated_at],
            )
            .map_err(|e| e.to_string())?;

        self.conn()
            .execute(
                "INSERT INTO sync_links (entity_type, entity_id, remote_id, dirty, deleted, updated_at, synced_at)
                 VALUES ('list', ?1, ?2, 0, 0, ?3, ?4)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                     remote_id = excluded.remote_id,
                     dirty = 0,
                     deleted = 0,
                     updated_at = excluded.updated_at,
                     synced_at = excluded.synced_at",
                params![list.id, remote_id, list.updated_at, now],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Upserts a task pulled from Google and marks its sync link clean.
    pub fn apply_remote_task(&self, task: &SyncTask, remote_id: &str) -> Result<(), String> {
        let now = now_ms();
        let encoded = match &task.repeat {
            Some(rule) => Some(serde_json::to_string(rule).map_err(|e| e.to_string())?),
            None => None,
        };

        self.conn()
            .execute(
                "INSERT INTO tasks (id, list_id, title, description, due_date, due_time, repeat_json,
                                    completed, next_occurrence, occurrence_index, position, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                     list_id = excluded.list_id,
                     title = excluded.title,
                     description = excluded.description,
                     due_date = excluded.due_date,
                     due_time = excluded.due_time,
                     repeat_json = excluded.repeat_json,
                     completed = excluded.completed,
                     next_occurrence = excluded.next_occurrence,
                     occurrence_index = excluded.occurrence_index,
                     position = excluded.position,
                     updated_at = excluded.updated_at",
                params![
                    task.id,
                    task.list_id,
                    task.title,
                    task.description,
                    task.due_date,
                    task.due_time,
                    encoded,
                    task.completed as i64,
                    task.next_occurrence,
                    task.occurrence_index,
                    task.position,
                    task.updated_at,
                ],
            )
            .map_err(|e| e.to_string())?;

        self.refresh_notification_state(&task.id)?;

        self.conn()
            .execute(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at)
                 VALUES ('task', ?1, ?2, ?3, 0, 0, ?4, ?5)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                     parent_id = excluded.parent_id,
                     remote_id = excluded.remote_id,
                     dirty = 0,
                     deleted = 0,
                     updated_at = excluded.updated_at,
                     synced_at = excluded.synced_at",
                params![task.id, task.list_id, remote_id, task.updated_at, now],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Removes a local list that was deleted remotely (without creating a dirty tombstone).
    pub fn remove_remote_deleted_list(&self, id: &str) -> Result<(), String> {
        let task_ids = self.task_ids_of(id)?;
        self.conn()
            .execute("DELETE FROM lists WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.drop_link(SyncEntity::List, id)?;
        for task_id in task_ids {
            self.drop_link(SyncEntity::Task, &task_id)?;
        }
        Ok(())
    }

    /// Removes a local task that was deleted remotely (without creating a dirty tombstone).
    pub fn remove_remote_deleted_task(&self, id: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        self.drop_link(SyncEntity::Task, id)?;
        Ok(())
    }

    /// All local list ids.
    pub fn all_list_ids(&self) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM lists")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// The ids of the tasks in a list.
    pub fn task_ids_of(&self, list_id: &str) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id FROM tasks WHERE list_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![list_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Records that a local entity changed and still has to be pushed.
    fn mark_dirty(
        &self,
        entity: SyncEntity,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), String> {
        let now = now_ms();
        self.conn()
            .execute(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at)
                 VALUES (?1, ?2, ?3, 1, 0, ?4)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                     parent_id = COALESCE(excluded.parent_id, sync_links.parent_id),
                     dirty = 1,
                     deleted = 0,
                     updated_at = excluded.updated_at",
                params![entity.as_str(), id, parent_id, now],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Records that an entity was deleted locally.
    ///
    /// The tombstone keeps the remote id, so the remote object can be
    /// deleted, and keeps the pull from resurrecting the entity.
    fn mark_deleted(
        &self,
        entity: SyncEntity,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), String> {
        let now = now_ms();
        self.conn()
            .execute(
                "INSERT INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at)
                 VALUES (?1, ?2, ?3, 1, 1, ?4)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                     parent_id = COALESCE(excluded.parent_id, sync_links.parent_id),
                     dirty = 1,
                     deleted = 1,
                     updated_at = excluded.updated_at",
                params![entity.as_str(), id, parent_id, now],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Stamps a local change: the entity's `updated_at` moves forward and the
    /// outbox learns that the change still has to be pushed.
    fn touch(&self, entity: SyncEntity, id: &str) -> Result<(), String> {
        let now = now_ms();
        let (table, parent_id) = match entity {
            SyncEntity::List => ("lists", None),
            SyncEntity::Task => ("tasks", self.task_list_id(id)?),
        };

        self.conn()
            .execute(
                &format!("UPDATE {table} SET updated_at = ?2 WHERE id = ?1"),
                params![id, now],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())?;

        self.mark_dirty(entity, id, parent_id.as_deref())
    }

    /// The outbox: every entity with local changes that are not pushed yet.
    ///
    /// Lists come first — a task cannot be pushed before the calendar it
    /// belongs to exists.
    pub fn pending_links(&self) -> Result<Vec<SyncLink>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at
                 FROM sync_links
                 WHERE dirty = 1
                 ORDER BY CASE entity_type WHEN 'list' THEN 0 ELSE 1 END, updated_at",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, i64>(5)? != 0,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut links = Vec::new();
        for row in rows {
            let (entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at) =
                row.map_err(|e| e.to_string())?;
            let Some(entity) = SyncEntity::parse(&entity_type) else {
                continue;
            };
            links.push(SyncLink {
                entity,
                entity_id,
                parent_id,
                remote_id,
                dirty,
                deleted,
                updated_at,
                synced_at,
            });
        }

        Ok(links)
    }

    /// How many local changes are waiting to be pushed.
    pub fn pending_count(&self) -> Result<i64, String> {
        self.conn()
            .query_row("SELECT COUNT(*) FROM sync_links WHERE dirty = 1", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())
    }

    /// The outbox row of one entity, if there is one.
    pub fn link(&self, entity: SyncEntity, id: &str) -> Result<Option<SyncLink>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at
                 FROM sync_links WHERE entity_type = ?1 AND entity_id = ?2",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![entity.as_str(), id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, i64>(5)? != 0,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(row) => {
                let (entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at) =
                    row.map_err(|e| e.to_string())?;
                Ok(Some(SyncLink {
                    entity: SyncEntity::parse(&entity_type).unwrap_or(entity),
                    entity_id,
                    parent_id,
                    remote_id,
                    dirty,
                    deleted,
                    updated_at,
                    synced_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Finds a sync link by its remote Google id.
    pub fn link_by_remote_id(
        &self,
        entity: SyncEntity,
        remote_id: &str,
    ) -> Result<Option<SyncLink>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at
                 FROM sync_links WHERE entity_type = ?1 AND remote_id = ?2",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query_map(params![entity.as_str(), remote_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, i64>(5)? != 0,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        match rows.next() {
            Some(row) => {
                let (entity_type, entity_id, parent_id, remote_id, dirty, deleted, updated_at, synced_at) =
                    row.map_err(|e| e.to_string())?;
                Ok(Some(SyncLink {
                    entity: SyncEntity::parse(&entity_type).unwrap_or(entity),
                    entity_id,
                    parent_id,
                    remote_id,
                    dirty,
                    deleted,
                    updated_at,
                    synced_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Remembers which Google object a local entity belongs to.
    pub fn set_remote_id(&self, entity: SyncEntity, id: &str, remote_id: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO sync_links (entity_type, entity_id, remote_id, dirty, updated_at)
                 VALUES (?1, ?2, ?3, 1, ?4)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET remote_id = excluded.remote_id",
                params![entity.as_str(), id, remote_id, now_ms()],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Marks an entity as pushed.
    pub fn mark_synced(&self, entity: SyncEntity, id: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "UPDATE sync_links SET dirty = 0, synced_at = ?3
                 WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity.as_str(), id, now_ms()],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Drops an outbox row — used once a remote delete is confirmed.
    pub fn drop_link(&self, entity: SyncEntity, id: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "DELETE FROM sync_links WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity.as_str(), id],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// When a local entity was last changed (unix milliseconds).
    pub fn entity_updated_at(&self, entity: SyncEntity, id: &str) -> Result<Option<i64>, String> {
        let table = match entity {
            SyncEntity::List => "lists",
            SyncEntity::Task => "tasks",
        };
        self.conn()
            .query_row(
                &format!("SELECT updated_at FROM {table} WHERE id = ?1"),
                params![id],
                |row| row.get::<_, i64>(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })
    }

    /// Reads a value from the small connection-state store.
    pub fn sync_state(&self, key: &str) -> Result<Option<String>, String> {
        self.conn()
            .query_row("SELECT value FROM sync_state WHERE key = ?1", params![key], |row| {
                row.get::<_, String>(0)
            })
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })
    }

    /// Writes a value into the connection-state store.
    pub fn set_sync_state(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT INTO sync_state (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Removes a value from the connection-state store.
    pub fn clear_sync_state(&self, key: &str) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM sync_state WHERE key = ?1", params![key])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Parses a `HH:MM` time; tasks without one are due at 09:00.
fn parse_time(value: Option<&str>) -> NaiveTime {
    value
        .and_then(|value| NaiveTime::parse_from_str(value, "%H:%M").ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"))
}

/// The moment a repeat occurrence is due.
fn occurrence_moment(date: NaiveDate, rule: &Repeat) -> NaiveDateTime {
    date.and_time(parse_time(rule.time.as_deref()))
}

/// Whether the series has run out of occurrences at `index`.
fn series_ended(rule: &Repeat, index: i64) -> bool {
    matches!(rule.end, RepeatEnd::After { count } if index >= count)
}

fn load_tasks(conn: &Connection, list_id: &str) -> Result<Vec<Task>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, description, due_date, due_time, repeat_json, completed,
                    next_occurrence, occurrence_index, notified_key
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
                completed: row.get::<_, i64>(6)? != 0,
                next_occurrence: row.get(7)?,
                occurrence_index: row.get(8)?,
                notified_key: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<Task>, _>>().map_err(|e| e.to_string())
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS lists (
             id         TEXT PRIMARY KEY,
             name       TEXT NOT NULL,
             color      TEXT NOT NULL,
             position   INTEGER NOT NULL,
             updated_at INTEGER NOT NULL DEFAULT 0
         );

         CREATE TABLE IF NOT EXISTS tasks (
             id          TEXT PRIMARY KEY,
             list_id     TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
             title       TEXT NOT NULL,
             description TEXT NOT NULL DEFAULT '',
             due_date    TEXT,
             due_time    TEXT,
             repeat_json TEXT,
             position    INTEGER NOT NULL,
             updated_at  INTEGER NOT NULL DEFAULT 0
         );

         CREATE INDEX IF NOT EXISTS idx_tasks_list_id ON tasks(list_id);

         -- The outbox, the id mapping and the tombstone log in one table.
         CREATE TABLE IF NOT EXISTS sync_links (
             entity_type TEXT    NOT NULL,
             entity_id   TEXT    NOT NULL,
             parent_id   TEXT,
             remote_id   TEXT,
             dirty       INTEGER NOT NULL DEFAULT 1,
             deleted     INTEGER NOT NULL DEFAULT 0,
             updated_at  INTEGER NOT NULL DEFAULT 0,
             synced_at   INTEGER NOT NULL DEFAULT 0,
             PRIMARY KEY (entity_type, entity_id)
         );

         -- Small key/value store for the connection state.
         CREATE TABLE IF NOT EXISTS sync_state (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )
    .map_err(|e| e.to_string())?;

    // Columns added after the first release — ALTER TABLE for existing files.
    ensure_columns(
        conn,
        "tasks",
        &[
            ("completed", "INTEGER NOT NULL DEFAULT 0"),
            ("next_occurrence", "TEXT"),
            ("occurrence_index", "INTEGER NOT NULL DEFAULT 0"),
            ("notified_key", "TEXT"),
            ("updated_at", "INTEGER NOT NULL DEFAULT 0"),
        ],
    )?;
    ensure_columns(conn, "lists", &[("updated_at", "INTEGER NOT NULL DEFAULT 0")])?;
    ensure_columns(conn, "sync_links", &[("parent_id", "TEXT")])?;

    // Everything that already existed when the sync was introduced has to be
    // queued once — otherwise the old lists and tasks would never reach
    // Google. `INSERT OR IGNORE` keeps the rows that are already there, so
    // this only ever adds what is missing.
    conn.execute_batch(
        "INSERT OR IGNORE INTO sync_links (entity_type, entity_id, dirty, deleted, updated_at)
         SELECT 'list', id, 1, 0, updated_at FROM lists;

         INSERT OR IGNORE INTO sync_links (entity_type, entity_id, parent_id, dirty, deleted, updated_at)
         SELECT 'task', id, list_id, 1, 0, updated_at FROM tasks;

         UPDATE sync_links
         SET parent_id = (SELECT list_id FROM tasks WHERE tasks.id = sync_links.entity_id)
         WHERE entity_type = 'task' AND parent_id IS NULL;",
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Adds the given columns to `table` if they are not there yet.
fn ensure_columns(
    conn: &Connection,
    table: &str,
    columns: &[(&str, &str)],
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?;
    let existing = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    for (name, declaration) in columns {
        if !existing.iter().any(|column| column == name) {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {name} {declaration}"),
                [],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// The current time in unix milliseconds — the stamp of a local change.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync;

    fn temp_db(name: &str) -> Db {
        let path = std::env::temp_dir().join(format!("checkd-test-{name}.db"));
        let _ = std::fs::remove_file(&path);
        Db::open_path(&path).expect("open temp db")
    }

    /// A local date and time, e.g. 2030-01-02 09:00.
    fn moment(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    fn daily_rule(start: &str) -> Repeat {
        Repeat {
            unit: RepeatUnit::Day,
            interval: 1,
            start: start.to_string(),
            time: Some("09:00".to_string()),
            weekdays: Vec::new(),
            month_repeat: None,
            end: RepeatEnd::Never,
        }
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

    #[test]
    fn completing_a_repeating_task_advances_to_the_next_occurrence() {
        let db = temp_db("advance");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Water plants").unwrap();

        let rule = daily_rule("2026-10-21");
        db.set_task_repeat("t1", Some(&rule)).unwrap();

        let task = db.get_task("t1").unwrap().unwrap();
        assert!(task.next_occurrence.is_some());
        let first = task.next_occurrence.clone().unwrap();

        let updated = db.set_task_completed("t1", true).unwrap();
        assert!(!updated.completed, "the next occurrence is pending again");
        assert_ne!(updated.next_occurrence, Some(first.clone()));
        assert_eq!(updated.occurrence_index, 1);
    }

    #[test]
    fn completing_the_last_occurrence_clears_the_rule() {
        let db = temp_db("series-end");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Once").unwrap();

        let mut rule = daily_rule("2026-10-21");
        rule.end = RepeatEnd::After { count: 1 };
        db.set_task_repeat("t1", Some(&rule)).unwrap();

        let updated = db.set_task_completed("t1", true).unwrap();
        assert!(updated.completed);
        assert!(updated.repeat.is_none(), "the series is over");
        assert!(updated.next_occurrence.is_none());
    }

    #[test]
    fn an_existing_database_gets_the_new_columns() {
        let path = std::env::temp_dir().join("checkd-test-migration.db");
        let _ = std::fs::remove_file(&path);

        // A database from before tasks could be completed.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE lists (
                     id TEXT PRIMARY KEY, name TEXT NOT NULL, color TEXT NOT NULL, position INTEGER NOT NULL
                 );
                 CREATE TABLE tasks (
                     id TEXT PRIMARY KEY,
                     list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
                     title TEXT NOT NULL,
                     description TEXT NOT NULL DEFAULT '',
                     due_date TEXT,
                     due_time TEXT,
                     repeat_json TEXT,
                     position INTEGER NOT NULL
                 );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO lists (id, name, color, position) VALUES ('l1', 'Old', '#fff', 1)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO tasks (id, list_id, title, position) VALUES ('t1', 'l1', 'Legacy', 1)",
                [],
            )
            .unwrap();
        }

        // Opening it with the current code migrates it in place.
        let db = Db::open_path(&path).unwrap();
        let data = db.load().unwrap();
        let task = &data.lists[0].tasks[0];

        assert_eq!(task.title, "Legacy");
        assert!(!task.completed);
        assert_eq!(task.next_occurrence, None);
        assert_eq!(task.occurrence_index, 0);

        // The old list and task are queued for the sync — they would
        // otherwise never reach Google.
        let pending = db.pending_links().unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].entity, SyncEntity::List);
        assert_eq!(pending[0].entity_id, "l1");
        assert_eq!(pending[1].entity, SyncEntity::Task);
        assert_eq!(pending[1].entity_id, "t1");
    }

    #[test]
    fn a_missed_occurrence_rolls_forward() {
        let db = temp_db("rollforward");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Water plants").unwrap();

        // A rule in the future keeps the test independent of the current date.
        let rule = daily_rule("2030-01-01");
        db.set_task_repeat("t1", Some(&rule)).unwrap();
        assert_eq!(
            db.get_task("t1").unwrap().unwrap().next_occurrence.as_deref(),
            Some("2030-01-01")
        );

        // The next occurrence is not due yet — nothing moves.
        let before = moment(2030, 1, 1, 8, 0);
        assert!(!db.roll_forward_missed(before).unwrap());
        assert_eq!(
            db.get_task("t1").unwrap().unwrap().next_occurrence.as_deref(),
            Some("2030-01-01")
        );

        // Once it is due, the missed occurrence is dropped and the series
        // continues — the reminders keep coming.
        let after = moment(2030, 1, 2, 9, 0);
        assert!(db.roll_forward_missed(after).unwrap());

        let task = db.get_task("t1").unwrap().unwrap();
        assert_eq!(task.next_occurrence.as_deref(), Some("2030-01-02"));
        assert_eq!(task.occurrence_index, 1);
        assert!(!task.completed);
        assert!(task.repeat.is_some(), "the series is still running");
    }

    #[test]
    fn rolling_forward_can_end_a_series() {
        let db = temp_db("rollforward-end");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Once a day").unwrap();

        let mut rule = daily_rule("2030-01-01");
        rule.end = RepeatEnd::After { count: 2 };
        db.set_task_repeat("t1", Some(&rule)).unwrap();

        // First missed occurrence — the series has one left.
        db.roll_forward_missed(moment(2030, 1, 2, 9, 0))
            .unwrap();
        let task = db.get_task("t1").unwrap().unwrap();
        assert_eq!(task.next_occurrence.as_deref(), Some("2030-01-02"));
        assert!(task.repeat.is_some());

        // Second one — the series is over.
        db.roll_forward_missed(moment(2030, 1, 3, 9, 0))
            .unwrap();
        let task = db.get_task("t1").unwrap().unwrap();
        assert!(task.completed);
        assert!(task.repeat.is_none(), "the series is over");
        assert_eq!(task.next_occurrence, None);
    }

    #[test]
    fn a_new_session_reminds_about_overdue_tasks_again() {
        let db = temp_db("reset-notifications");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();

        // The reminder for this occurrence went out in an earlier session.
        db.mark_notified("t1", "due:2026-10-21").unwrap();
        let tasks = db.tasks_for_notifications().unwrap();
        assert_eq!(
            tasks[0].task.notified_key.as_deref(),
            Some("due:2026-10-21")
        );

        // Launching the app starts a fresh session — the task is mentioned
        // again the next time the app is opened.
        db.reset_notifications().unwrap();
        let tasks = db.tasks_for_notifications().unwrap();
        assert_eq!(tasks[0].task.notified_key, None);
    }

    // --- sync bookkeeping ---------------------------------------------------

    #[test]
    fn creating_a_list_queues_it_for_the_sync() {
        let db = temp_db("outbox-create");

        db.create_list("l1", "Groceries", "#3b82f6").unwrap();

        let pending = db.pending_links().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].entity, SyncEntity::List);
        assert_eq!(pending[0].entity_id, "l1");
        assert!(pending[0].dirty);
        assert!(!pending[0].deleted);
        assert_eq!(pending[0].remote_id, None);
        assert!(pending[0].updated_at > 0);
    }

    #[test]
    fn the_outbox_puts_lists_before_their_tasks() {
        let db = temp_db("outbox-order");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();

        let pending = db.pending_links().unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].entity, SyncEntity::List);
        assert_eq!(pending[1].entity, SyncEntity::Task);
    }

    #[test]
    fn a_change_after_a_push_queues_the_entity_again() {
        let db = temp_db("outbox-redirty");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.set_remote_id(SyncEntity::List, "l1", "cal-1").unwrap();
        db.mark_synced(SyncEntity::List, "l1").unwrap();
        assert_eq!(db.pending_count().unwrap(), 0);

        let before = db.entity_updated_at(SyncEntity::List, "l1").unwrap().unwrap();
        db.rename_list("l1", "Renamed").unwrap();
        let after = db.entity_updated_at(SyncEntity::List, "l1").unwrap().unwrap();

        assert!(after >= before);
        assert_eq!(db.pending_count().unwrap(), 1);

        let link = db.link(SyncEntity::List, "l1").unwrap().unwrap();
        assert!(link.dirty);
        // The remote id is kept — the push updates the same calendar.
        assert_eq!(link.remote_id.as_deref(), Some("cal-1"));
    }

    #[test]
    fn deleting_a_task_leaves_a_tombstone() {
        let db = temp_db("outbox-tombstone");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();
        db.mark_synced(SyncEntity::List, "l1").unwrap();
        db.set_remote_id(SyncEntity::Task, "t1", "evt-1").unwrap();
        db.mark_synced(SyncEntity::Task, "t1").unwrap();

        db.delete_task("t1").unwrap();

        assert!(db.load().unwrap().lists[0].tasks.is_empty());

        let link = db.link(SyncEntity::Task, "t1").unwrap().unwrap();
        assert!(link.deleted);
        assert!(link.dirty);
        assert_eq!(link.remote_id.as_deref(), Some("evt-1"));
        assert_eq!(db.pending_count().unwrap(), 1);
    }

    #[test]
    fn deleting_a_list_tombstones_its_tasks_too() {
        let db = temp_db("outbox-list-tombstone");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();
        db.create_task("t2", "l1", "Bread").unwrap();

        db.delete_list("l1").unwrap();

        let list = db.link(SyncEntity::List, "l1").unwrap().unwrap();
        assert!(list.deleted);
        for id in ["t1", "t2"] {
            let task = db.link(SyncEntity::Task, id).unwrap().unwrap();
            assert!(task.deleted, "task {id} needs a tombstone");
        }
        assert_eq!(db.pending_count().unwrap(), 3);
    }

    #[test]
    fn completing_a_task_queues_it() {
        let db = temp_db("outbox-complete");
        db.create_list("l1", "List", "#3b82f6").unwrap();
        db.create_task("t1", "l1", "Milk").unwrap();
        db.mark_synced(SyncEntity::List, "l1").unwrap();
        db.mark_synced(SyncEntity::Task, "t1").unwrap();
        assert_eq!(db.pending_count().unwrap(), 0);

        db.set_task_completed("t1", true).unwrap();

        assert_eq!(db.pending_count().unwrap(), 1);
        let link = db.link(SyncEntity::Task, "t1").unwrap().unwrap();
        assert!(link.dirty);
        assert!(!link.deleted);
    }

    #[test]
    fn the_connection_state_round_trips() {
        let db = temp_db("outbox-state");
        assert_eq!(db.sync_state(sync::STATE_ENABLED).unwrap(), None);

        db.set_sync_state(sync::STATE_ENABLED, "1").unwrap();
        db.set_sync_state(sync::STATE_ACCOUNT, "user@example.com").unwrap();

        assert_eq!(
            db.sync_state(sync::STATE_ENABLED).unwrap().as_deref(),
            Some("1")
        );
        assert_eq!(
            db.sync_state(sync::STATE_ACCOUNT).unwrap().as_deref(),
            Some("user@example.com")
        );
    }

    #[test]
    fn due_moment_uses_the_occurrence_for_repeating_tasks() {
        let task = Task {
            id: "t1".into(),
            title: "Milk".into(),
            description: String::new(),
            due_date: None,
            due_time: None,
            repeat: Some(daily_rule("2026-10-21")),
            completed: false,
            next_occurrence: Some("2026-10-21".into()),
            occurrence_index: 0,
            notified_key: None,
        };

        let (moment, key) = task.due_moment().unwrap();
        assert_eq!(moment.to_string(), "2026-10-21 09:00:00");
        assert_eq!(key, "occ:2026-10-21");
    }
}
