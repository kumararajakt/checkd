import { useEffect, useState } from "react";
import type { Repeat, RepeatUnit } from "../types";

type RepeatModalProps = {
  repeat: Repeat | null;
  /** Start date to prefill when creating a new rule (usually the task's due date). */
  defaultStart: string;
  /** Time to prefill when creating a new rule (usually the task's due time). */
  defaultTime: string | null;
  onSave: (repeat: Repeat | null) => void;
  onClose: () => void;
};

type EndType = Repeat["end"]["type"];

const UNITS: { value: RepeatUnit; label: string }[] = [
  { value: "day", label: "Day" },
  { value: "week", label: "Week" },
  { value: "month", label: "Month" },
  { value: "year", label: "Year" },
];

/** Short weekday labels, index 0 = Sunday (matches Date#getDay()). */
const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Full weekday names, index 0 = Sunday. */
const WEEKDAY_NAMES = [
  "Sunday",
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
];

const DAYS_OF_MONTH = Array.from({ length: 31 }, (_, i) => i + 1);

function plural(unit: RepeatUnit, count: number): string {
  return count === 1 ? unit : `${unit}s`;
}

export default function RepeatModal({
  repeat,
  defaultStart,
  defaultTime,
  onSave,
  onClose,
}: RepeatModalProps) {
  const [unit, setUnit] = useState<RepeatUnit>(repeat?.unit ?? "day");
  const [interval, setInterval] = useState(String(repeat?.interval ?? 1));
  const [start, setStart] = useState(repeat?.start ?? defaultStart);
  const [time, setTime] = useState(repeat?.time ?? defaultTime ?? "");
  const [weekdays, setWeekdays] = useState<number[]>(repeat?.weekdays ?? []);

  const initialMonth = repeat?.monthRepeat ?? null;
  const initialDay =
    initialMonth?.type === "weekday"
      ? 1
      : initialMonth?.type === "day"
        ? initialMonth.day
        : initialMonth?.type === "last-day"
          ? -1
          : Number((repeat?.start ?? defaultStart).slice(8, 10)) || 1;
  const [monthMode, setMonthMode] = useState<"day" | "weekday">(
    initialMonth?.type === "weekday" ? "weekday" : "day",
  );
  const [monthDay, setMonthDay] = useState(initialDay === -1 ? "last" : String(initialDay));
  const [monthWeekday, setMonthWeekday] = useState(
    initialMonth?.type === "weekday" ? initialMonth.weekday : 0,
  );
  const [monthWeek, setMonthWeek] = useState<1 | 2 | 3 | 4 | "last">(
    initialMonth?.type === "weekday" ? initialMonth.week : 1,
  );

  const initialEnd = repeat?.end ?? { type: "never" as const };
  const [endType, setEndType] = useState<EndType>(initialEnd.type);
  const [endDate, setEndDate] = useState(
    initialEnd.type === "on" ? initialEnd.date : defaultStart,
  );
  const [endCount, setEndCount] = useState(
    initialEnd.type === "after" ? String(initialEnd.count) : "1",
  );

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  function save() {
    const n = Math.max(1, Math.floor(Number(interval)) || 1);
    onSave({
      unit,
      interval: n,
      start: start || defaultStart,
      time: time || null,
      // Only weekly rules care about weekdays; drop them otherwise.
      weekdays:
        unit === "week" ? [...weekdays].sort((a, b) => a - b) : [],
      monthRepeat:
        unit !== "month"
          ? null
          : monthMode === "day"
            ? monthDay === "last"
              ? { type: "last-day" }
              : { type: "day", day: Number(monthDay) }
            : { type: "weekday", weekday: monthWeekday, week: monthWeek },
      end:
        endType === "never"
          ? { type: "never" }
          : endType === "on"
            ? { type: "on", date: endDate || start || defaultStart }
            : {
                type: "after",
                count: Math.max(1, Math.floor(Number(endCount)) || 1),
              },
    });
    onClose();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="w-96 rounded-xl bg-white p-4 shadow-xl dark:bg-gray-900"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Repeat options"
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="font-semibold">Repeat</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-100"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
              className="size-5"
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="grid grid-cols-4 gap-1 rounded-md bg-gray-100 p-1 dark:bg-gray-800">
          {UNITS.map((u) => (
            <button
              key={u.value}
              type="button"
              onClick={() => setUnit(u.value)}
              className={`rounded px-2 py-1 text-xs font-medium ${
                unit === u.value
                  ? "bg-white text-gray-900 shadow-sm dark:bg-gray-700 dark:text-gray-100"
                  : "text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
              }`}
            >
              {u.label}
            </button>
          ))}
        </div>

        {unit === "week" && (
          <div className="mt-3">
            <div className="mb-1.5 flex items-center justify-between">
              <span className="text-xs text-gray-500 dark:text-gray-400">
                On {weekdays.length > 1 ? "(multiple days)" : ""}
              </span>
              {weekdays.length > 0 && (
                <button
                  type="button"
                  onClick={() => setWeekdays([])}
                  className="text-xs text-gray-400 hover:text-gray-700 dark:hover:text-gray-100"
                >
                  Use start date’s day
                </button>
              )}
            </div>
            <div className="grid grid-cols-7 gap-1">
              {WEEKDAYS.map((label, day) => {
                const active = weekdays.includes(day);
                return (
                  <button
                    key={label}
                    type="button"
                    onClick={() =>
                      setWeekdays((prev) =>
                        prev.includes(day)
                          ? prev.filter((d) => d !== day)
                          : [...prev, day],
                      )
                    }
                    title={label}
                    aria-pressed={active}
                    className={`rounded-md py-1 text-xs font-medium ${
                      active
                        ? "bg-blue-600 text-white"
                        : "border border-gray-300 text-gray-600 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-800"
                    }`}
                  >
                    {label}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {unit === "month" && (
          <div className="mt-3 flex flex-col gap-2 text-sm text-gray-700 dark:text-gray-200">
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="month-mode"
                checked={monthMode === "day"}
                onChange={() => setMonthMode("day")}
              />
              <span>On day</span>
              <select
                value={monthDay}
                disabled={monthMode !== "day"}
                onChange={(e) => setMonthDay(e.currentTarget.value)}
                className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
              >
                {DAYS_OF_MONTH.map((d) => (
                  <option key={d} value={d}>
                    {d}
                  </option>
                ))}
                <option value="last">Last day</option>
              </select>
            </label>

            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="month-mode"
                checked={monthMode === "weekday"}
                onChange={() => setMonthMode("weekday")}
              />
              <span>On the</span>
              <select
                value={String(monthWeek)}
                disabled={monthMode !== "weekday"}
                onChange={(e) =>
                  setMonthWeek(
                    e.currentTarget.value === "last"
                      ? "last"
                      : (Number(e.currentTarget.value) as 1 | 2 | 3 | 4),
                  )
                }
                className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
              >
                <option value="1">first</option>
                <option value="2">second</option>
                <option value="3">third</option>
                <option value="4">fourth</option>
                <option value="last">last</option>
              </select>
              <select
                value={String(monthWeekday)}
                disabled={monthMode !== "weekday"}
                onChange={(e) => setMonthWeekday(Number(e.currentTarget.value))}
                className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
              >
                {WEEKDAY_NAMES.map((name, day) => (
                  <option key={name} value={day}>
                    {name}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}

        <div className="mt-4 flex items-center gap-2 text-sm text-gray-700 dark:text-gray-200">
          <span>Every</span>
          <input
            type="number"
            min={1}
            value={interval}
            onChange={(e) => setInterval(e.currentTarget.value)}
            aria-label="Repeat interval"
            className="w-16 rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
          />
          <span>{plural(unit, Math.max(1, Math.floor(Number(interval)) || 1))}</span>
        </div>

        <div className="mt-2 flex items-center gap-2 text-sm text-gray-700 dark:text-gray-200">
          <span>At</span>
          <input
            type="time"
            value={time}
            onChange={(e) => setTime(e.currentTarget.value)}
            aria-label="Repeat time"
            className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
          />
        </div>

        <label className="mt-3 flex items-center justify-between gap-2 text-sm text-gray-700 dark:text-gray-200">
          Starts
          <input
            type="date"
            value={start}
            onChange={(e) => setStart(e.currentTarget.value)}
            className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
          />
        </label>

        <fieldset className="mt-4">
          <legend className="mb-2 text-sm font-medium text-gray-700 dark:text-gray-200">Ends</legend>
          <div className="flex flex-col gap-2">
            <label className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-200">
              <input
                type="radio"
                name="repeat-end"
                checked={endType === "never"}
                onChange={() => setEndType("never")}
              />
              Never
            </label>

            <label className="flex items-center justify-between gap-2 text-sm text-gray-700 dark:text-gray-200">
              <span className="flex items-center gap-2">
                <input
                  type="radio"
                  name="repeat-end"
                  checked={endType === "on"}
                  onChange={() => setEndType("on")}
                />
                On
              </span>
              <input
                type="date"
                value={endDate}
                disabled={endType !== "on"}
                onChange={(e) => setEndDate(e.currentTarget.value)}
                className="rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
              />
            </label>

            <label className="flex items-center justify-between gap-2 text-sm text-gray-700 dark:text-gray-200">
              <span className="flex items-center gap-2">
                <input
                  type="radio"
                  name="repeat-end"
                  checked={endType === "after"}
                  onChange={() => setEndType("after")}
                />
                After
              </span>
              <span className="flex items-center gap-2">
                <input
                  type="number"
                  min={1}
                  value={endCount}
                  disabled={endType !== "after"}
                  onChange={(e) => setEndCount(e.currentTarget.value)}
                  aria-label="Number of occurrences"
                  className="w-16 rounded-md border border-gray-300 px-2 py-1 text-sm outline-none focus:border-blue-400 disabled:opacity-40 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
                />
                <span className="text-xs text-gray-500 dark:text-gray-400">occurrences</span>
              </span>
            </label>
          </div>
        </fieldset>

        <div className="mt-5 flex items-center justify-between">
          {repeat ? (
            <button
              type="button"
              onClick={() => {
                onSave(null);
                onClose();
              }}
              className="text-sm text-red-500 hover:text-red-600"
            >
              Remove repeat
            </button>
          ) : (
            <span />
          )}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-50"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={save}
              className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
            >
              Save
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
