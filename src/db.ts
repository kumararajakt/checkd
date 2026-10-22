/**
 * Typed wrappers around the SQLite-backed Rust commands.
 *
 * The React state is the source of truth for rendering, but every mutation is
 * written through to the database here. Writes are optimistic: the UI updates
 * immediately and `persist` resyncs from the database if a write fails.
 */
import { invoke } from "@tauri-apps/api/core";
import type { AppData, CredentialsStatus, Repeat, SyncStatus, Task } from "./types";

export const db = {
  load: () => invoke<AppData>("load_data"),

  createList: (id: string, name: string, color: string) =>
    invoke<void>("create_list", { id, name, color }),

  renameList: (id: string, name: string) =>
    invoke<void>("rename_list", { id, name }),

  deleteList: (id: string) => invoke<void>("delete_list", { id }),

  setListColor: (id: string, color: string) =>
    invoke<void>("set_list_color", { id, color }),

  createTask: (id: string, listId: string, title: string) =>
    invoke<void>("create_task", { id, listId, title }),

  renameTask: (id: string, title: string) =>
    invoke<void>("rename_task", { id, title }),

  setTaskDue: (id: string, dueDate: string | null, dueTime: string | null) =>
    invoke<void>("set_task_due", { id, dueDate, dueTime }),

  setTaskDescription: (id: string, description: string) =>
    invoke<void>("set_task_description", { id, description }),

  /**
   * Sets the repeat rule. The returned task carries the new state — the
   * active occurrence is anchored to the rule.
   */
  setTaskRepeat: (id: string, repeat: Repeat | null) =>
    invoke<Task>("set_task_repeat", { id, repeat }),

  /**
   * Marks a task done. The returned task carries the new state — a repeating
   * task has already advanced to its next occurrence.
   */
  setTaskCompleted: (id: string, completed: boolean) =>
    invoke<Task>("set_task_completed", { id, completed }),

  deleteTask: (id: string) => invoke<void>("delete_task", { id }),

  /** The state of the Google Calendar sync. */
  googleStatus: () => invoke<SyncStatus>("google_status"),

  /** The OAuth client the app found in the environment. */
  googleCredentialsStatus: () =>
    invoke<CredentialsStatus>("google_credentials_status"),

  /** Opens the consent screen and connects the account. */
  googleSignIn: () => invoke<SyncStatus>("google_sign_in"),

  /** Gives the access back to Google and forgets the tokens. */
  googleSignOut: () => invoke<SyncStatus>("google_sign_out"),

  /** Talks to Google now. */
  googleSyncNow: () => invoke<SyncStatus>("google_sync_now"),
};
