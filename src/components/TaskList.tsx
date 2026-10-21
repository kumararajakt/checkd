import { useState, type FormEvent } from "react";
import type { List, Repeat } from "../types";
import TaskItem from "./TaskItem";

type TaskListProps = {
  list: List;
  onAdd: (listId: string, title: string) => void;
  onRename: (listId: string, taskId: string, title: string) => void;
  onDelete: (listId: string, taskId: string) => void;
  onSetDueDate: (listId: string, taskId: string, dueDate: string | null) => void;
  onSetDueTime: (listId: string, taskId: string, dueTime: string | null) => void;
  onSetDescription: (listId: string, taskId: string, description: string) => void;
  onSetRepeat: (listId: string, taskId: string, repeat: Repeat | null) => void;
};

export default function TaskList({
  list,
  onAdd,
  onRename,
  onDelete,
  onSetDueDate,
  onSetDueTime,
  onSetDescription,
  onSetRepeat,
}: TaskListProps) {
  const [draft, setDraft] = useState("");

  function submitNewTask(e: FormEvent) {
    e.preventDefault();
    const title = draft.trim();
    if (!title) return;
    onAdd(list.id, title);
    setDraft("");
  }

  return (
    <div className="mx-auto w-full max-w-xl">
      <h1 className="flex items-center gap-3 text-2xl font-bold">
        <span
          style={{ backgroundColor: list.color }}
          className="size-4 shrink-0 rounded-full"
        />
        <span className="truncate">{list.name}</span>
      </h1>

      <form onSubmit={submitNewTask} className="mt-4">
        <input
          value={draft}
          onChange={(e) => setDraft(e.currentTarget.value)}
          placeholder="+ New Task"
          className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 outline-none focus:border-blue-500"
        />
      </form>

      <div className="mt-4 flex flex-col gap-1">
        {list.tasks.length === 0 ? (
          <p className="px-3 text-sm text-gray-400">No tasks yet.</p>
        ) : (
          list.tasks.map((task) => (
            <TaskItem
              key={task.id}
              task={task}
              onRename={(taskId, title) => onRename(list.id, taskId, title)}
              onDelete={(taskId) => onDelete(list.id, taskId)}
              onSetDueDate={(taskId, dueDate) =>
                onSetDueDate(list.id, taskId, dueDate)
              }
              onSetDueTime={(taskId, dueTime) =>
                onSetDueTime(list.id, taskId, dueTime)
              }
              onSetDescription={(taskId, description) =>
                onSetDescription(list.id, taskId, description)
              }
              onSetRepeat={(taskId, repeat) => onSetRepeat(list.id, taskId, repeat)}
            />
          ))
        )}
      </div>
    </div>
  );
}
