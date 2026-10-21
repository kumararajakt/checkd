import { useState, type FormEvent } from "react";
import type { List } from "../types";

type SidebarProps = {
  lists: List[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: (name: string) => void;
};

const SETTINGS = {
  label: "Settings",
  icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
};

function NavButton({
  item,
  expanded,
  active = false,
  onClick,
}: {
  item: { label: string; icon?: string };
  expanded: boolean;
  active?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={item.label}
      className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 ${
        active ? "bg-gray-100 font-medium" : ""
      } ${expanded ? "justify-start" : "justify-center"}`}
    >
      {item.icon ? (
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={1.5}
          stroke="currentColor"
          className="size-5 shrink-0"
        >
          <path strokeLinecap="round" strokeLinejoin="round" d={item.icon} />
        </svg>
      ) : (
        <span className="size-4 shrink-0 rounded-full bg-blue-500" />
      )}
      {expanded && <span className="truncate">{item.label}</span>}
    </button>
  );
}

export default function Sidebar({ lists, selectedId, onSelect, onAdd }: SidebarProps) {
  const [expanded, setExpanded] = useState(true);
  const [draft, setDraft] = useState("");

  function submitNewList(e: FormEvent) {
    e.preventDefault();
    const name = draft.trim();
    if (!name) return;
    onAdd(name);
    setDraft("");
  }

  return (
    <aside
      className={`relative flex h-full shrink-0 flex-col border-r border-gray-200 bg-white transition-[width] duration-200 ease-in-out ${
        expanded ? "w-1/3" : "w-16"
      }`}
    >
      <div className="flex h-12 items-center gap-2 border-b border-gray-200 px-4">
        <img src="/checkd.svg" alt="checkd" className="size-6 shrink-0" />
        {expanded && <span className="truncate font-semibold">checkd</span>}
      </div>

      <form onSubmit={submitNewList} className="border-b border-gray-200 p-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.currentTarget.value)}
          placeholder={expanded ? "+ New List" : "+"}
          className={`w-full rounded-md bg-transparent px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 outline-none hover:bg-gray-100 focus:bg-gray-100 ${
            expanded ? "" : "text-center"
          }`}
        />
      </form>

      <nav className="flex flex-1 flex-col gap-1 overflow-y-auto p-2">
        {lists.map((list) => (
          <NavButton
            key={list.id}
            item={{ label: list.name }}
            expanded={expanded}
            active={list.id === selectedId}
            onClick={() => onSelect(list.id)}
          />
        ))}
      </nav>

      <div className="border-t border-gray-200 p-2">
        <NavButton item={SETTINGS} expanded={expanded} />
      </div>

      {/* Expand/collapse toggle pinned to the sidebar's outer edge */}
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        aria-label={expanded ? "Collapse sidebar" : "Expand sidebar"}
        className="absolute top-16 -right-3 flex size-6 items-center justify-center rounded-full border border-gray-200 bg-white text-gray-500 shadow-sm hover:bg-gray-50"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
          strokeWidth={2}
          stroke="currentColor"
          className={`size-4 transition-transform duration-200 ${expanded ? "" : "rotate-180"}`}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5" />
        </svg>
      </button>
    </aside>
  );
}
