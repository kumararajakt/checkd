import { useState, type KeyboardEvent } from "react";
import type { Task } from "../types";

type TaskItemProps = {
  task: Task;
  onRename: (id: string, title: string) => void;
  onDelete: (id: string) => void;
};

const TRASH_ICON =
  "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0";

export default function TaskItem({ task, onRename, onDelete }: TaskItemProps) {
  const [editing, setEditing] = useState(false);
  const [editDraft, setEditDraft] = useState("");

  function startEdit() {
    setEditDraft(task.title);
    setEditing(true);
  }

  function commitEdit() {
    setEditing(false);
    const title = editDraft.trim();
    if (title && title !== task.title) onRename(task.id, title);
  }

  function editKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") commitEdit();
    if (e.key === "Escape") setEditing(false);
  }

  return (
    <div className="flex items-center gap-3 rounded-md px-3 py-2 hover:bg-gray-100">
      {editing ? (
        <input
          autoFocus
          value={editDraft}
          onChange={(e) => setEditDraft(e.currentTarget.value)}
          onKeyDown={editKeyDown}
          onBlur={commitEdit}
          className="min-w-0 flex-1 rounded-md border border-blue-400 bg-white px-3 py-1.5 text-sm text-gray-900 outline-none"
        />
      ) : (
        <button
          type="button"
          onDoubleClick={startEdit}
          title={`${task.title} (double-click to rename)`}
          className="min-w-0 flex-1 text-left text-sm text-gray-800"
        >
          <span className="block truncate">{task.title}</span>
        </button>
      )}

      <button
        type="button"
        onClick={() => onDelete(task.id)}
        onDoubleClick={(e) => e.stopPropagation()}
        aria-label={`Delete ${task.title}`}
        title="Delete task"
        className="shrink-0 p-1 text-gray-400 hover:text-red-500"
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
}
