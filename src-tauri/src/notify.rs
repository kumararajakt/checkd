//! Desktop notifications for tasks that come due.
//!
//! A background thread polls the database, fires a notification when a task
//! reaches its due moment, and sleeps until the next moment is due. It runs
//! for the whole life of the app — including while the window is hidden in
//! the tray.

use std::thread;
use std::time::Duration;

use chrono::{Local, NaiveDateTime};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::{Db, Task, TaskWithList};

/// Event sent to the frontend when the scheduler changed the data.
pub const DATA_CHANGED: &str = "data-changed";

/// Longest the scheduler sleeps before looking at the database again — data
/// can change (a new task, a new due date) at any time.
const MAX_SLEEP: Duration = Duration::from_secs(60);

/// Starts the notification scheduler.
pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        // Every launch starts a fresh reminder session: everything that is
        // overdue is mentioned again, no matter how long ago it was due.
        if let Some(db) = app.try_state::<Db>() {
            if let Err(err) = db.reset_notifications() {
                eprintln!("could not reset the notification state: {err}");
            }
        }

        loop {
            let wait = tick(&app);
            thread::sleep(wait);
        }
    });
}

/// Fires every notification that is due and returns how long to wait before
/// the next check.
fn tick(app: &AppHandle) -> Duration {
    let now = Local::now().naive_local();
    let Some(db) = app.try_state::<Db>() else {
        return MAX_SLEEP;
    };

    // A repeating task whose occurrence was missed moves on, so the series
    // keeps reminding instead of going silent.
    match db.roll_forward_missed(now) {
        Ok(true) => {
            let _ = app.emit(DATA_CHANGED, ());
        }
        Ok(false) => {}
        Err(err) => eprintln!("could not roll repeating tasks forward: {err}"),
    }

    let Ok(tasks) = db.tasks_for_notifications() else {
        return MAX_SLEEP;
    };

    let plan = plan(&tasks, now);

    for (id, key, task, list_name) in plan.due {
        let Some((moment, _)) = task.due_moment() else {
            continue;
        };
        if send(app, &list_name, &task, moment, now) {
            let _ = db.mark_notified(&id, &key);
        }
    }

    match plan.next {
        Some(moment) => {
            let millis = (moment - now).num_milliseconds().max(0) as u64;
            Duration::from_millis(millis.min(MAX_SLEEP.as_millis() as u64))
        }
        None => MAX_SLEEP,
    }
}

/// What the scheduler has to do at a given moment.
pub struct Plan {
    /// The reminders that are due: task id, notification key, task, list name.
    pub due: Vec<(String, String, Task, String)>,
    /// The moment of the next reminder.
    pub next: Option<NaiveDateTime>,
}

/// Works out which reminders are due and when the next one is.
pub fn plan(tasks: &[TaskWithList], now: NaiveDateTime) -> Plan {
    let mut due = Vec::new();
    let mut next: Option<NaiveDateTime> = None;

    for entry in tasks {
        let Some((moment, key)) = entry.task.due_moment() else {
            continue;
        };

        if moment > now {
            // Still to come — wake up exactly when it is due.
            next = Some(match next {
                Some(current) if current <= moment => current,
                _ => moment,
            });
            continue;
        }

        // The moment has passed: remind the user about it once per occurrence.
        // Reminders already sent in this session are skipped, so nothing
        // repeats while the app is running.
        if entry.task.notified_key.as_deref() == Some(key.as_str()) {
            continue;
        }

        due.push((
            entry.task.id.clone(),
            key,
            entry.task.clone(),
            entry.list_name.clone(),
        ));
    }

    Plan { due, next }
}

/// Shows a desktop notification. Returns whether it was delivered.
fn send(
    app: &AppHandle,
    list_name: &str,
    task: &Task,
    moment: NaiveDateTime,
    now: NaiveDateTime,
) -> bool {
    let when = moment.format("%b %-d, %Y · %H:%M");
    let body = if moment < now {
        format!("Overdue · {list_name} · {when}")
    } else {
        format!("{list_name} · {when}")
    };

    app.notification()
        .builder()
        .title(&task.title)
        .body(body)
        .show()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Repeat, RepeatEnd, RepeatUnit};

    fn task_with(key: Option<&str>) -> Task {
        Task {
            id: "t1".into(),
            title: "Milk".into(),
            description: String::new(),
            due_date: Some("2026-10-21".into()),
            due_time: Some("18:00".into()),
            repeat: None,
            completed: false,
            next_occurrence: None,
            occurrence_index: 0,
            notified_key: key.map(str::to_string),
        }
    }

    fn entry(task: Task) -> TaskWithList {
        TaskWithList {
            task,
            list_name: "Groceries".into(),
        }
    }

    fn at(value: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn a_one_off_task_is_due_at_its_due_moment() {
        let (moment, key) = task_with(None).due_moment().unwrap();
        assert_eq!(moment.to_string(), "2026-10-21 18:00:00");
        assert_eq!(key, "due:2026-10-21T18:00");
    }

    #[test]
    fn a_completed_task_is_never_due() {
        let mut task = task_with(None);
        task.completed = true;
        assert!(task.due_moment().is_none());
    }

    #[test]
    fn a_task_without_a_time_is_due_in_the_morning() {
        let mut task = task_with(None);
        task.due_time = None;
        let (moment, key) = task.due_moment().unwrap();
        assert_eq!(moment.to_string(), "2026-10-21 09:00:00");
        assert_eq!(key, "due:2026-10-21");
    }

    #[test]
    fn a_repeating_task_is_due_at_its_active_occurrence() {
        let mut task = task_with(None);
        task.repeat = Some(Repeat {
            unit: RepeatUnit::Day,
            interval: 1,
            start: "2026-10-21".into(),
            time: Some("07:15".into()),
            weekdays: Vec::new(),
            month_repeat: None,
            end: RepeatEnd::Never,
        });
        task.next_occurrence = Some("2026-10-24".into());

        let (moment, key) = task.due_moment().unwrap();
        assert_eq!(moment.to_string(), "2026-10-24 07:15:00");
        assert_eq!(key, "occ:2026-10-24");
    }

    #[test]
    fn a_missed_reminder_is_caught_up_once() {
        let now = at("2026-10-21 19:00");
        let tasks = vec![entry(task_with(None))];

        let schedule = plan(&tasks, now);
        assert_eq!(schedule.due.len(), 1);
        assert_eq!(schedule.due[0].1, "due:2026-10-21T18:00");

        // Once it has been sent, the same occurrence is not repeated.
        let tasks = vec![entry(task_with(Some("due:2026-10-21T18:00")))];
        assert!(plan(&tasks, now).due.is_empty());
    }

    #[test]
    fn an_old_overdue_task_is_reminded_too() {
        // A task that has been due for days is still worth mentioning when
        // the app is opened.
        let now = at("2026-10-25 09:00");
        let tasks = vec![entry(task_with(None))];

        let schedule = plan(&tasks, now);
        assert_eq!(schedule.due.len(), 1);
        assert_eq!(schedule.due[0].1, "due:2026-10-21T18:00");
    }

    #[test]
    fn a_future_task_is_not_due_but_schedules_the_next_check() {
        let now = at("2026-10-21 09:00");
        let tasks = vec![entry(task_with(None))];

        let plan = plan(&tasks, now);
        assert!(plan.due.is_empty());
        assert_eq!(plan.next, Some(at("2026-10-21 18:00")));
    }

    #[test]
    fn the_earliest_moment_wins() {
        let now = at("2026-10-21 09:00");

        let mut later = task_with(None);
        later.id = "t2".into();
        later.due_time = Some("20:00".into());

        let mut earlier = task_with(None);
        earlier.id = "t3".into();
        earlier.due_time = Some("10:00".into());

        let tasks = vec![entry(later), entry(earlier)];
        let plan = plan(&tasks, now);
        assert_eq!(plan.next, Some(at("2026-10-21 10:00")));
    }
}
