import { useState, type KeyboardEvent } from "react";
import type { Repeat, Task } from "../types";
import RepeatModal from "./RepeatModal";

type TaskItemProps = {
  task: Task;
  onRename: (id: string, title: string) => void;
  onDelete: (id: string) => void;
  onSetDueDate: (id: string, dueDate: string | null) => void;
  onSetDueTime: (id: string, dueTime: string | null) => void;
  onSetDescription: (id: string, description: string) => void;
  onSetRepeat: (id: string, repeat: Repeat | null) => void;
};

const TRASH_ICON =
  "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0";

const CHEVRON_ICON = "M8.25 4.5l7.5 7.5-7.5 7.5";

const CALENDAR_ICON =
  "M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5";

const REPEAT_ICON =
  "M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99";

/** Format YYYY-MM-DD as e.g. "Oct 24, 2026" (parsed as a local date, not UTC). */
function formatDueDate(dueDate: string): string {
  const [y, m, d] = dueDate.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

/** Format HH:MM as e.g. "5:30 PM". */
function formatDueTime(dueTime: string): string {
  const [h, m] = dueTime.split(":").map(Number);
  return new Date(2000, 0, 1, h, m).toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
  });
}

/** Today as YYYY-MM-DD in local time (string-comparable with stored due dates). */
function todayString(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** Current time as HH:MM in local time (string-comparable with stored due times). */
function nowTimeString(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(now.getHours())}:${pad(now.getMinutes())}`;
}

/**
 * Overdue (negative/red) when the date is before today; for a date equal to
 * today, overdue once the due time has passed (or not yet, if no time set).
 */
function isOverdue(task: Task): boolean {
  if (!task.dueDate) return false;
  const today = todayString();
  if (task.dueDate !== today) return task.dueDate < today;
  return task.dueTime !== null && task.dueTime < nowTimeString();
}

/** Combined label like "Oct 24, 2026 · 5:30 PM". */
function dueLabel(task: Task): string {
  if (!task.dueDate) return "";
  const date = formatDueDate(task.dueDate);
  return task.dueTime ? `${date} · ${formatDueTime(task.dueTime)}` : date;
}

/** Long weekday name for 0 = Sunday … 6 = Saturday (Jan 2, 2000 was a Sunday). */
function weekdayName(day: number): string {
  return new Date(2000, 0, 2 + day).toLocaleDateString(undefined, {
    weekday: "long",
  });
}

/** "first" | "second" | "third" | "fourth" | "last" */
function weekLabel(week: 1 | 2 | 3 | 4 | "last"): string {
  return week === "last" ? "last" : ["first", "second", "third", "fourth"][week - 1];
}

/** Short human summary, e.g. "Every 2 weeks on Sunday · until Oct 24, 2026". */
function repeatSummary(repeat: Repeat): string {
  const base =
    repeat.interval === 1
      ? `Every ${repeat.unit}`
      : `Every ${repeat.interval} ${repeat.unit}s`;
  let summary = base;
  if (repeat.unit === "week" && repeat.weekday !== null) {
    summary += ` on ${weekdayName(repeat.weekday)}`;
  }
  if (repeat.unit === "month" && repeat.monthRepeat) {
    const m = repeat.monthRepeat;
    if (m.type === "day") summary += ` on day ${m.day}`;
    else if (m.type === "last-day") summary += ` on the last day`;
    else summary += ` on the ${weekLabel(m.week)} ${weekdayName(m.weekday)}`;
  }
  if (repeat.time) summary += ` at ${formatDueTime(repeat.time)}`;
  if (repeat.end.type === "on") {
    return `${summary} · until ${formatDueDate(repeat.end.date)}`;
  }
  if (repeat.end.type === "after") {
    return `${summary} · ${repeat.end.count}×`;
  }
  return summary;
}

export default function TaskItem({
  task,
  onRename,
  onDelete,
  onSetDueDate,
  onSetDueTime,
  onSetDescription,
  onSetRepeat,
}: TaskItemProps) {
  const [editing, setEditing] = useState(false);
  const [editDraft, setEditDraft] = useState("");
  const [open, setOpen] = useState(false);
  const [picking, setPicking] = useState(false);
  const [showRepeatModal, setShowRepeatModal] = useState(false);

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
    <div className="rounded-md hover:bg-gray-100">
      <div className="flex items-center gap-3 px-3 py-2">
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
            onClick={() => setOpen((v) => !v)}
            onDoubleClick={startEdit}
            title={`${task.title} (click to expand, double-click to rename)`}
            className="flex min-w-0 flex-1 items-start gap-2 text-left text-sm text-gray-800"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
              className={`mt-1 size-3.5 shrink-0 text-gray-400 transition-transform duration-200 ${
                open ? "rotate-90" : ""
              }`}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d={CHEVRON_ICON} />
            </svg>
            <span className="min-w-0 flex-1">
              <span className="block truncate">{task.title}</span>
              {task.dueDate && (
                <span
                  className={`mt-1 inline-block rounded-full px-2 py-0.5 text-xs font-medium ${
                    isOverdue(task)
                      ? "bg-red-50 text-red-600"
                      : "bg-gray-100 text-gray-600"
                  }`}
                >
                  {dueLabel(task)}
                </span>
              )}
            </span>
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

      {/* Accordion panel: due date controls + description */}
      <div
        className={`grid transition-[grid-template-rows] duration-200 ease-in-out ${
          open ? "grid-rows-[1fr]" : "grid-rows-[0fr]"
        }`}
      >
        <div className="overflow-hidden">
          <div className="flex items-center gap-2 px-3 pb-2 pl-9">
            {picking ? (
              <input
                type="date"
                autoFocus
                value={task.dueDate ?? ""}
                onChange={(e) => {
                  onSetDueDate(task.id, e.currentTarget.value || null);
                  setPicking(false);
                }}
                onBlur={() => setPicking(false)}
                className="rounded-md border border-blue-400 bg-white px-2 py-1 text-xs text-gray-900 outline-none"
              />
            ) : (
              <button
                type="button"
                onClick={() => setPicking(true)}
                className={`flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs ${
                  !task.dueDate
                    ? "border-gray-300 text-gray-600 hover:bg-white"
                    : isOverdue(task)
                      ? "border-red-200 bg-red-50 text-red-600"
                      : "border-blue-200 bg-blue-50 text-blue-700"
                }`}
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  strokeWidth={1.5}
                  stroke="currentColor"
                  className="size-3.5"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" d={CALENDAR_ICON} />
                </svg>
                {task.dueDate ? dueLabel(task) : "Set due date"}
              </button>
            )}

            {task.dueDate && (
              <input
                type="time"
                value={task.dueTime ?? ""}
                onChange={(e) => onSetDueTime(task.id, e.currentTarget.value || null)}
                aria-label="Due time"
                className="rounded-md border border-gray-300 bg-white px-2 py-1 text-xs text-gray-700 outline-none focus:border-blue-400"
              />
            )}

            {task.dueDate && !picking && (
              <button
                type="button"
                onClick={() => onSetDueDate(task.id, null)}
                className="text-xs text-gray-400 hover:text-red-500"
              >
                Clear
              </button>
            )}
          </div>

          <div className="px-3 pb-2 pl-9">
            <textarea
              value={task.description}
              onChange={(e) => onSetDescription(task.id, e.currentTarget.value)}
              placeholder="Add a description..."
              rows={2}
              className="w-full resize-none rounded-md border border-gray-300 bg-white px-2 py-1 text-xs text-gray-700 outline-none focus:border-blue-400"
            />
          </div>

          <div className="px-3 pb-2 pl-9">
            <button
              type="button"
              onClick={() => setShowRepeatModal(true)}
              className={`flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs ${
                task.repeat
                  ? "border-blue-200 bg-blue-50 text-blue-700"
                  : "border-gray-300 text-gray-600 hover:bg-white"
              }`}
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                strokeWidth={1.5}
                stroke="currentColor"
                className="size-3.5"
              >
                <path strokeLinecap="round" strokeLinejoin="round" d={REPEAT_ICON} />
              </svg>
              {task.repeat ? repeatSummary(task.repeat) : "Repeat"}
            </button>
          </div>
        </div>
      </div>

      {showRepeatModal && (
        <RepeatModal
          repeat={task.repeat}
          defaultStart={task.dueDate ?? todayString()}
          defaultTime={task.dueTime}
          onSave={(repeat) => onSetRepeat(task.id, repeat)}
          onClose={() => setShowRepeatModal(false)}
        />
      )}
    </div>
  );
}
