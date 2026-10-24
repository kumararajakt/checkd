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

  setTaskRepeat: (id: string, repeat: Repeat | null) =>
    invoke<Task>("set_task_repeat", { id, repeat }),

  setTaskCompleted: (id: string, completed: boolean) =>
    invoke<Task>("set_task_completed", { id, completed }),

  deleteTask: (id: string) => invoke<void>("delete_task", { id }),

  googleStatus: () => invoke<SyncStatus>("google_status"),

  googleCredentialsStatus: () =>
    invoke<CredentialsStatus>("google_credentials_status"),

  googleSignIn: () => invoke<SyncStatus>("google_sign_in"),

  googleSignOut: () => invoke<SyncStatus>("google_sign_out"),

  googleSyncNow: () => invoke<SyncStatus>("google_sync_now"),
};
