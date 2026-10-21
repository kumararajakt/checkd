import { useState } from "react";
import Sidebar from "./components/Sidebar";

function App() {
  const [sidebarExpanded, setSidebarExpanded] = useState(true);

  return (
    <div className="flex h-screen overflow-hidden bg-gray-50 text-gray-900">
      <Sidebar
        expanded={sidebarExpanded}
        onToggle={() => setSidebarExpanded((v) => !v)}
      />

      <main className="flex-1 overflow-y-auto p-6">
        <h1 className="text-2xl font-bold">Todos</h1>
        <p className="text-gray-500">Your todo app starts here.</p>
      </main>
    </div>
  );
}

export default App;
