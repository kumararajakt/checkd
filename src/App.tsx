import { useState } from "react";
import Sidebar from "./components/Sidebar";
import type { List } from "./types";

function App() {
  const [lists, setLists] = useState<List[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  function addList(name: string) {
    const list: List = { id: crypto.randomUUID(), name };
    setLists((prev) => [...prev, list]);
    setSelectedId(list.id);
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
      />

      <main className="flex-1 overflow-y-auto p-6">
        {selected ? (
          <>
            <h1 className="text-2xl font-bold">{selected.name}</h1>
            <p className="text-gray-500">Nothing here yet.</p>
          </>
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
