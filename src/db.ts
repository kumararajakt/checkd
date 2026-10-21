/**
 * Typed wrappers around the SQLite-backed Rust commands.
 *
 * The React state is the source of truth for rendering, but every mutation is
 * written through to the database here. Writes are optimistic: the UI updates
 * immediately and `persist` resyncs from the database if a write fails.
 */
import { invoke } from "@tauri-apps/api/core";
import type { AppData, Repeat } from "./types";

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

  setTaskRepeat: (id: string, repeat: Repeat | null) =>
    invoke<void>("set_task_repeat", { id, repeat }),

  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
};
