import { useEffect, useState } from "react";

type TimeModalProps = {
  /** Current due time as `HH:MM` (24h), or null when the task has no time. */
  time: string | null;
  /** Called with the picked time — null drops the time again. */
  onSave: (time: string | null) => void;
  onClose: () => void;
};

/** Quick picks offered above the time picker. */
const PRESETS = [
  { label: "Morning", time: "09:00" },
  { label: "Noon", time: "12:00" },
  { label: "Afternoon", time: "15:00" },
  { label: "Evening", time: "18:00" },
  { label: "Night", time: "21:00" },
];

/** Tasks without a time of their own count as due at 09:00. */
const DEFAULT_TIME = "09:00";

export default function TimeModal({ time, onSave, onClose }: TimeModalProps) {
  const [value, setValue] = useState(time ?? DEFAULT_TIME);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  function save() {
    onSave(value || null);
    onClose();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="w-72 rounded-xl bg-white p-4 shadow-xl dark:bg-gray-900"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Due time"
      >
        <div className="mb-3 flex items-center justify-between">
          <h2 className="font-semibold">Due time</h2>
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

        <div className="flex justify-center">
          <input
            type="time"
            value={value}
            onChange={(e) => setValue(e.currentTarget.value)}
            aria-label="Due time"
            className="rounded-md border border-gray-300 bg-white px-3 py-2 text-2xl font-medium text-gray-900 outline-none focus:border-blue-400 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
          />
        </div>

        <div className="mt-3 flex flex-wrap justify-center gap-1">
          {PRESETS.map((preset) => (
            <button
              key={preset.time}
              type="button"
              onClick={() => setValue(preset.time)}
              aria-pressed={value === preset.time}
              className={`rounded-full px-2.5 py-1 text-xs font-medium ${
                value === preset.time
                  ? "bg-blue-600 text-white"
                  : "bg-gray-100 text-gray-600 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700"
              }`}
            >
              {preset.label}
            </button>
          ))}
        </div>

        <div className="mt-5 flex items-center justify-between">
          {time ? (
            <button
              type="button"
              onClick={() => {
                onSave(null);
                onClose();
              }}
              className="text-sm text-red-500 hover:text-red-600"
            >
              Clear time
            </button>
          ) : (
            <span />
          )}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 dark:text-gray-200"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={save}
              className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
            >
              OK
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
