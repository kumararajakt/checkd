import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save as saveFileDialog } from "@tauri-apps/plugin-dialog";
import Sidebar from "./components/Sidebar";
import SettingsDialog from "./components/SettingsDialog";
import TaskList from "./components/TaskList";
import { applyTheme, loadSettings, saveSettings, watchSystemTheme } from "./settings";
import type { Settings } from "./settings";
import type { List, Repeat } from "./types";

function App() {
  const [lists, setLists] = useState<List[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(loadSettings);
  const [showSettings, setShowSettings] = useState(false);

  function addList(name: string) {
    const list: List = { id: crypto.randomUUID(), name, color: "#3b82f6", tasks: [] };
    setLists((prev) => [...prev, list]);
    setSelectedId(list.id);
  }

  function updateTasks(listId: string, update: (tasks: List["tasks"]) => List["tasks"]) {
    setLists((prev) =>
      prev.map((l) => (l.id === listId ? { ...l, tasks: update(l.tasks) } : l)),
    );
  }

  function addTask(listId: string, title: string) {
    updateTasks(listId, (tasks) => [
      ...tasks,
      {
        id: crypto.randomUUID(),
        title,
        description: "",
        dueDate: null,
        dueTime: null,
        repeat: null,
      },
    ]);
  }

  function renameTask(listId: string, taskId: string, title: string) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, title } : t)),
    );
  }

  function deleteTask(listId: string, taskId: string) {
    updateTasks(listId, (tasks) => tasks.filter((t) => t.id !== taskId));
  }

  function setTaskDueDate(listId: string, taskId: string, dueDate: string | null) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) =>
        t.id === taskId
          ? // Time is only valid alongside a date — drop it when the date goes.
            { ...t, dueDate, dueTime: dueDate ? t.dueTime : null }
          : t,
      ),
    );
  }

  function setTaskDueTime(listId: string, taskId: string, dueTime: string | null) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, dueTime } : t)),
    );
  }

  function setTaskDescription(listId: string, taskId: string, description: string) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, description } : t)),
    );
  }

  function setTaskRepeat(listId: string, taskId: string, repeat: Repeat | null) {
    updateTasks(listId, (tasks) =>
      tasks.map((t) => (t.id === taskId ? { ...t, repeat } : t)),
    );
  }

  function setListColor(id: string, color: string) {
    setLists((prev) => prev.map((l) => (l.id === id ? { ...l, color } : l)));
  }

  function renameList(id: string, name: string) {
    setLists((prev) => prev.map((l) => (l.id === id ? { ...l, name } : l)));
  }

  function deleteList(id: string) {
    setLists((prev) => prev.filter((l) => l.id !== id));
    setSelectedId((prev) => (prev === id ? null : prev));
  }

  const selected = lists.find((l) => l.id === selectedId);

  // Theme: apply the chosen theme, and follow the OS when set to "system".
  useEffect(() => {
    applyTheme(settings.theme);
    if (settings.theme !== "system") return;
    return watchSystemTheme(() => applyTheme("system"));
  }, [settings.theme]);

  useEffect(() => {
    saveSettings(settings);
  }, [settings]);

  // Tell the Rust side whether closing the window should hide to the tray.
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
        onSelect={setSelectedId}
        onAdd={addList}
        onRename={renameList}
        onDelete={deleteList}
        onSetColor={setListColor}
        onOpenSettings={() => setShowSettings(true)}
      />

      <main className="flex-1 overflow-y-auto p-6">
        {selected ? (
          <TaskList
            list={selected}
            onAdd={addTask}
            onRename={renameTask}
            onDelete={deleteTask}
            onSetDueDate={setTaskDueDate}
            onSetDueTime={setTaskDueTime}
            onSetDescription={setTaskDescription}
            onSetRepeat={setTaskRepeat}
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
    </div>
  );
}

export default App;
