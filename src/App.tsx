import { useState } from "react";
import Sidebar from "./components/Sidebar";
import TaskList from "./components/TaskList";
import type { List } from "./types";

function App() {
  const [lists, setLists] = useState<List[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);

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

  return (
    <div className="flex h-screen overflow-hidden bg-gray-50 text-gray-900">
      <Sidebar
        lists={lists}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onAdd={addList}
        onRename={renameList}
        onDelete={deleteList}
        onSetColor={setListColor}
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
          />
        ) : (
          <>
            <h1 className="text-2xl font-bold">checkd</h1>
            <p className="text-gray-500">Create a list with “+ New List” to get started.</p>
          </>
        )}
      </main>
    </div>
  );
}

export default App;
