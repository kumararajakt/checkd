import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import Sidebar from "./components/Sidebar";
import SettingsDialog from "./components/SettingsDialog";
import TaskList from "./components/TaskList";
import { db } from "./db";
import { applyTheme, loadSettings, saveSettings, watchSystemTheme } from "./settings";
import type { Settings } from "./settings";
import type { List, Repeat, Task } from "./types";

const CLOCK_TICK_MS = 30_000;

function App() {
  const [lists, setLists] = useState<List[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(loadSettings);
  const [showSettings, setShowSettings] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    db.load()
      .then((data) => {
        setLists(data.lists);
        const last = loadSettings().lastListId;
        if (!last) return;
        if (data.lists.some((l) => l.id === last)) {
          setSelectedId(last);
        } else {
          updateSettings({ lastListId: null });
        }
      })
      .catch((err) => setError(`Could not load data: ${err}`))
      .finally(() => setLoading(false));

    db.googleStatus()
      .then((status) => {
        if (status.connected) {
          db.googleSyncNow().catch(() => {});
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), CLOCK_TICK_MS);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    isPermissionGranted()
      .then((granted) => (granted ? null : requestPermission()))
      .catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = listen("data-changed", () => {
      db.load()
        .then((data) => setLists(data.lists))
        .catch(() => {});
    });
    return () => {
      unlisten.then((off) => off()).catch(() => {});
    };
  }, []);

  function persist(write: Promise<unknown>): Promise<unknown> {
    return write
      .then(() => {
        db.googleStatus()
          .then((status) => {
            if (status.connected) {
              db.googleSyncNow().catch(() => {});
            }
          })
          .catch(() => {});
      })
      .catch(async (err) => {
        setError(`Could not save changes: ${err}`);
        try {
          const data = await db.load();
          setLists(data.lists);
        } catch {
        }
      });
  }

  function addList(name: string) {
    const list: List = { id: crypto.randomUUID(), name, color: "#3b82f6", tasks: [] };
    setLists((prev) => [...prev, list]);
    setSelectedId(list.id);
    updateSettings({ lastListId: list.id });
    persist(db.createList(list.id, list.name, list.color));
  }

  function updateTasks(listId: string, update: (tasks: List["tasks"]) => List["tasks"]) {
    setLists((prev) =>
      prev.map((l) => (l.id === listId ? { ...l, tasks: update(l.tasks) } : l)),
    );
  }

  function findTask(listId: string, taskId: string): Task | undefined {
    return lists.find((l) => l.id === listId)?.tasks.find((t) => t.id === taskId);
  }

  function addTask(listId: string, title: string) {
    const task: Task = {
      id: crypto.randomUUID(),
      title,
      description: "",
      dueDate: null,
      dueTime: null,
      repeat: null,
      completed: false,
      nextOccurrence: null,
      occurrenceIndex: 0,
    };
    updateTasks(listId, (tasks) => [...tasks, task]);
    persist(db.createTask(task.id, listId, task.title));
  }

  function renameTask(listId: string, taskId: string, title: string) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, title } : t)),
    );
    persist(db.renameTask(taskId, title));
  }

  function deleteTask(listId: string, taskId: string) {
    updateTasks(listId, (tasks) => tasks.filter((t) => t.id !== taskId));
    persist(db.deleteTask(taskId));
  }

  function setTaskDueDate(listId: string, taskId: string, dueDate: string | null) {
    const dueTime = dueDate ? (findTask(listId, taskId)?.dueTime ?? null) : null;
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, dueDate, dueTime } : t)),
    );
    persist(db.setTaskDue(taskId, dueDate, dueTime));
  }

  function setTaskDueTime(listId: string, taskId: string, dueTime: string | null) {
    const dueDate = findTask(listId, taskId)?.dueDate ?? null;
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, dueTime } : t)),
    );
    persist(db.setTaskDue(taskId, dueDate, dueTime));
  }

  function setTaskDescription(listId: string, taskId: string, description: string) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, description } : t)),
    );
    persist(db.setTaskDescription(taskId, description));
  }

  function setTaskRepeat(listId: string, taskId: string, repeat: Repeat | null) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, repeat } : t)),
    );
    persist(
      db.setTaskRepeat(taskId, repeat).then((updated) => {
        updateTasks(listId, (tasks) =>
          tasks.map((t) => (t.id === taskId ? updated : t)),
        );
      }),
    );
  }

  function setTaskCompleted(listId: string, taskId: string, completed: boolean) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, completed } : t)),
    );
    persist(
      db.setTaskCompleted(taskId, completed).then((updated) => {
        updateTasks(listId, (tasks) =>
          tasks.map((t) => (t.id === taskId ? updated : t)),
        );
      }),
    );
  }

  function setListColor(id: string, color: string) {
    setLists((prev) => prev.map((l) => (l.id === id ? { ...l, color } : l)));
    persist(db.setListColor(id, color));
  }

  function renameList(id: string, name: string) {
    setLists((prev) => prev.map((l) => (l.id === id ? { ...l, name } : l)));
    persist(db.renameList(id, name));
  }

  function deleteList(id: string) {
    setLists((prev) => prev.filter((l) => l.id !== id));
    if (selectedId === id) {
      setSelectedId(null);
      updateSettings({ lastListId: null });
    }
    persist(db.deleteList(id));
  }

  function selectList(id: string) {
    setSelectedId(id);
    updateSettings({ lastListId: id });
  }

  const selected = lists.find((l) => l.id === selectedId);

  useEffect(() => {
    applyTheme(settings.theme);
    if (settings.theme !== "system") return;
    return watchSystemTheme(() => applyTheme("system"));
  }, [settings.theme]);

  useEffect(() => {
    saveSettings(settings);
  }, [settings]);

  useEffect(() => {
    invoke("set_close_to_tray", { enabled: settings.closeToTray }).catch(() => {});
  }, [settings.closeToTray]);

  function updateSettings(patch: Partial<Settings>) {
    setSettings((prev) => ({ ...prev, ...patch }));
  }

  async function exportData(): Promise<string> {
    const path = await saveFileDialog({
      defaultPath: "checkd-export.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return "Export cancelled.";
    const contents = JSON.stringify({ lists }, null, 2);
    await invoke("export_data", { path, contents });
    return `Exported to ${path}`;
  }

  return (
    <div className="flex h-screen overflow-hidden bg-gray-50 text-gray-900 dark:bg-gray-950 dark:text-gray-100">
      <Sidebar
        lists={lists}
        selectedId={selectedId}
        onSelect={selectList}
        onAdd={addList}
        onRename={renameList}
        onDelete={deleteList}
        onSetColor={setListColor}
        onOpenSettings={() => setShowSettings(true)}
      />

      <main className="flex-1 overflow-y-auto p-6">
        {loading ? (
          <p className="text-gray-500 dark:text-gray-400">Loading…</p>
        ) : selected ? (
          <TaskList
            list={selected}
            now={now}
            onAdd={addTask}
            onRename={renameTask}
            onDelete={deleteTask}
            onSetDueDate={setTaskDueDate}
            onSetDueTime={setTaskDueTime}
            onSetDescription={setTaskDescription}
            onSetRepeat={setTaskRepeat}
            onSetCompleted={setTaskCompleted}
          />
        ) : (
          <>
            <h1 className="text-2xl font-bold">checkd</h1>
            <p className="text-gray-500 dark:text-gray-400">
              Create a list with “+ New List” to get started.
            </p>
          </>
        )}
      </main>

      {showSettings && (
        <SettingsDialog
          settings={settings}
          onChange={updateSettings}
          onExport={exportData}
          onClose={() => setShowSettings(false)}
        />
      )}

      {error && (
        <div className="fixed right-4 bottom-4 flex max-w-sm items-start gap-3 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 shadow-lg dark:border-red-900 dark:bg-red-950 dark:text-red-300">
          <span className="flex-1">{error}</span>
          <button
            type="button"
            onClick={() => setError(null)}
            aria-label="Dismiss"
            className="shrink-0 text-red-400 hover:text-red-600 dark:hover:text-red-200"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
              className="size-4"
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}

export default App;
