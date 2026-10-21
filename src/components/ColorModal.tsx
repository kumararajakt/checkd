import { useEffect } from "react";

type ColorModalProps = {
  color: string;
  onPick: (color: string) => void;
  onClose: () => void;
};

const COLORS = [
  "#3b82f6", // blue
  "#ef4444", // red
  "#22c55e", // green
  "#eab308", // yellow
  "#f97316", // orange
  "#ec4899", // pink
  "#8b5cf6", // violet
  "#14b8a6", // teal
  "#64748b", // slate
  "#1f2937", // gray
];

export default function ColorModal({ color, onPick, onClose }: ColorModalProps) {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

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
        aria-label="Change list color"
      >
        <div className="mb-3 flex items-center justify-between">
          <h2 className="font-semibold">List color</h2>
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

        <div className="grid grid-cols-5 gap-2">
          {COLORS.map((c) => (
            <button
              key={c}
              type="button"
              onClick={() => {
                onPick(c);
                onClose();
              }}
              style={{ backgroundColor: c }}
              aria-label={`Color ${c}`}
              className={`size-8 rounded-full hover:scale-110 ${
                c === color ? "ring-2 ring-gray-900 ring-offset-2" : ""
              }`}
            />
          ))}
        </div>

        <label className="mt-4 flex items-center justify-between text-sm text-gray-600 dark:text-gray-400">
          Custom
          <input
            type="color"
            value={color}
            onChange={(e) => onPick(e.currentTarget.value)}
            className="size-8 cursor-pointer rounded-md border-0 bg-transparent p-0"
          />
        </label>
      </div>
    </div>
  );
}
