import { useState, type FormEvent, type KeyboardEvent } from "react";
import type { List } from "../types";
import ColorModal from "./ColorModal";

type SidebarProps = {
  lists: List[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: (name: string) => void;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onSetColor: (id: string, color: string) => void;
};

const SETTINGS = {
  label: "Settings",
  icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
};

const TRASH_ICON =
  "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0";

export default function Sidebar({
  lists,
  selectedId,
  onSelect,
  onAdd,
  onRename,
  onDelete,
  onSetColor,
}: SidebarProps) {
  const [expanded, setExpanded] = useState(true);
  const [draft, setDraft] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState("");
  const [colorForId, setColorForId] = useState<string | null>(null);

  function submitNewList(e: FormEvent) {
    e.preventDefault();
    const name = draft.trim();
    if (!name) return;
    onAdd(name);
    setDraft("");
  }

  function startEdit(list: List) {
    setEditingId(list.id);
    setEditDraft(list.name);
  }

  function commitEdit(list: List) {
    setEditingId(null);
    const name = editDraft.trim();
    if (name && name !== list.name) onRename(list.id, name);
  }

  function editKeyDown(list: List, e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") commitEdit(list);
    if (e.key === "Escape") setEditingId(null);
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
        {lists.map((list) => {
          const active = list.id === selectedId;
          const editing = list.id === editingId;
          return (
            <div
              key={list.id}
              className={`group flex items-center rounded-md hover:bg-gray-100 ${
                active ? "bg-gray-100" : ""
              }`}
            >
              {editing ? (
                <input
                  autoFocus
                  value={editDraft}
                  onChange={(e) => setEditDraft(e.currentTarget.value)}
                  onKeyDown={(e) => editKeyDown(list, e)}
                  onBlur={() => commitEdit(list)}
                  className={`min-w-0 flex-1 rounded-md border border-blue-400 bg-white px-3 py-1.5 text-sm text-gray-900 outline-none ${
                    expanded ? "" : "w-full text-center"
                  }`}
                />
              ) : (
                <button
                  type="button"
                  onClick={() => onSelect(list.id)}
                  onDoubleClick={() => startEdit(list)}
                  title={`${list.name} (double-click to rename)`}
                  className={`flex min-w-0 flex-1 items-center gap-3 py-2 text-sm text-gray-700 ${
                    expanded ? "px-3" : "flex-col gap-1 px-1"
                  }`}
                >
                  <span
                    style={{ backgroundColor: list.color }}
                    onDoubleClick={(e) => {
                      e.stopPropagation();
                      setEditingId(null);
                      setColorForId(list.id);
                    }}
                    title="Double-click to change color"
                    className={`shrink-0 cursor-pointer rounded-full ${
                      expanded ? "size-4" : "size-3"
                    }`}
                  />
                  {expanded && <span className="truncate text-left">{list.name}</span>}
                </button>
              )}

              <button
                type="button"
                onClick={() => onDelete(list.id)}
                aria-label={`Delete ${list.name}`}
                title="Delete list"
                className={`shrink-0 p-1 text-gray-400 hover:text-red-500 ${
                  expanded ? "" : "pb-2"
                }`}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  strokeWidth={1.5}
                  stroke="currentColor"
                  className="size-4"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" d={TRASH_ICON} />
                </svg>
              </button>
            </div>
          );
        })}
      </nav>

      <div className="border-t border-gray-200 p-2">
        <button
          type="button"
          title={SETTINGS.label}
          className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm text-gray-700 hover:bg-gray-100 ${
            expanded ? "justify-start" : "justify-center"
          }`}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={1.5}
            stroke="currentColor"
            className="size-5 shrink-0"
          >
            <path strokeLinecap="round" strokeLinejoin="round" d={SETTINGS.icon} />
          </svg>
          {expanded && <span className="truncate">{SETTINGS.label}</span>}
        </button>
      </div>

      {/* Expand/collapse toggle pinned to the sidebar's outer edge */}      <button
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

      {colorForId &&
        (() => {
          const list = lists.find((l) => l.id === colorForId);
          if (!list) return null;
          return (
            <ColorModal
              color={list.color}
              onPick={(color) => onSetColor(list.id, color)}
              onClose={() => setColorForId(null)}
            />
          );
        })()}
    </aside>
  );
}
