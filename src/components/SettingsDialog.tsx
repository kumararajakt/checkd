import { useEffect, useState } from "react";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { check as checkUpdate } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { db } from "../db";
import type { Settings, Theme } from "../settings";
import type { SyncStatus } from "../types";

type SettingsDialogProps = {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onExport: () => Promise<string>;
  onClose: () => void;
};

const THEMES: { value: Theme; label: string; icon: string }[] = [
  {
    value: "system",
    label: "System",
    icon: "M9 17.25v1.007a3 3 0 01-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0118 18.257V17.25m6-12V15a2.25 2.25 0 01-2.25 2.25H5.25A2.25 2.25 0 013 15V5.25m18 0A2.25 2.25 0 0018.75 3H5.25A2.25 2.25 0 003 5.25m18 0V12a2.25 2.25 0 01-2.25 2.25H5.25A2.25 2.25 0 013 12V5.25",
  },
  {
    value: "light",
    label: "Light",
    icon: "M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z",
  },
  {
    value: "dark",
    label: "Dark",
    icon: "M21.752 15.002A9.72 9.72 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 009.002-5.998z",
  },
];

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function GoogleSyncSection() {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;

    (async () => {
      try {
        const current = await db.googleStatus();
        if (!alive) return;
        setStatus(current);
      } catch (e) {
        if (alive) setError(messageOf(e));
      }
    })();

    return () => {
      alive = false;
    };
  }, []);

  async function run(action: () => Promise<SyncStatus>) {
    setBusy(true);
    setError(null);
    try {
      const result = await action();
      setStatus(result);
      if (action === db.googleSignIn) {
        try {
          setStatus(await db.googleSyncNow());
        } catch (e) {
          setError(messageOf(e));
          try {
            setStatus(await db.googleStatus());
          } catch {
          }
        }
      }
    } catch (e) {
      setError(messageOf(e));
      try {
        setStatus(await db.googleStatus());
      } catch {
      }
    } finally {
      setBusy(false);
    }
  }

  const connected = status?.connected ?? false;

  return (
    <section>
      <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
        Sync
      </h3>

      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center gap-2">
          {connected ? (
            <button
              type="button"
              onClick={() => run(db.googleSignOut)}
              disabled={busy}
              className="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
            >
              Sign out
            </button>
          ) : (
            <button
              type="button"
              onClick={() => run(db.googleSignIn)}
              disabled={busy}
              className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {busy ? "Waiting for Google…" : "Sign in"}
            </button>
          )}
        </div>

        {(error ?? status?.lastError) && (
          <p className="text-xs text-red-600 dark:text-red-400">
            {error ?? status?.lastError}
          </p>
        )}
      </div>
    </section>
  );
}

export default function SettingsDialog({
  settings,
  onChange,
  onExport,
  onClose,
}: SettingsDialogProps) {
  const [exporting, setExporting] = useState(false);
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const [autostart, setAutostart] = useState(false);
  const [autostartLoading, setAutostartLoading] = useState(true);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    isAutostartEnabled()
      .then((enabled) => {
        if (alive) setAutostart(enabled);
      })
      .catch(() => {})
      .finally(() => {
        if (alive) setAutostartLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  async function handleAutostartChange(enable: boolean) {
    setAutostart(enable);
    try {
      if (enable) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
    } catch {
      try {
        setAutostart(await isAutostartEnabled());
      } catch {
        setAutostart(!enable);
      }
    }
  }

  async function handleCheckUpdate() {
    setCheckingUpdate(true);
    setUpdateStatus(null);
    try {
      const update = await checkUpdate();
      if (update) {
        setUpdateStatus(`Downloading ${update.version}…`);
        await update.downloadAndInstall();
        setUpdateStatus(`Restarting to apply ${update.version}…`);
        await relaunch();
      } else {
        setUpdateStatus("checkd is up to date.");
      }
    } catch (e) {
      setUpdateStatus(messageOf(e));
    } finally {
      setCheckingUpdate(false);
    }
  }

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  async function handleExport() {
    setExporting(true);
    setExportStatus(null);
    try {
      setExportStatus(await onExport());
    } catch (e) {
      setExportStatus(e instanceof Error ? e.message : String(e));
    } finally {
      setExporting(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="max-h-[90vh] w-96 overflow-y-auto rounded-xl bg-white p-4 shadow-xl dark:bg-gray-900"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Settings"
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="font-semibold">Settings</h2>
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

        <div className="flex flex-col gap-5">
          <section>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
              Appearance
            </h3>
            <div className="grid grid-cols-3 gap-1 rounded-md bg-gray-100 p-1 dark:bg-gray-800">
              {THEMES.map((t) => (
                <button
                  key={t.value}
                  type="button"
                  onClick={() => onChange({ theme: t.value })}
                  aria-pressed={settings.theme === t.value}
                  className={`flex items-center justify-center gap-1.5 rounded px-2 py-1.5 text-xs font-medium ${
                    settings.theme === t.value
                      ? "bg-white text-gray-900 shadow-sm dark:bg-gray-700 dark:text-gray-100"
                      : "text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-200"
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
                    <path strokeLinecap="round" strokeLinejoin="round" d={t.icon} />
                  </svg>
                  {t.label}
                </button>
              ))}
            </div>
          </section>

          <section>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
              Window
            </h3>
            <label className="flex items-start gap-3 text-sm text-gray-700 dark:text-gray-200">
              <input
                type="checkbox"
                checked={settings.closeToTray}
                onChange={(e) => onChange({ closeToTray: e.currentTarget.checked })}
                className="mt-0.5 size-4 accent-blue-600"
              />
              <span>
                Close to tray
                <span className="block text-xs text-gray-500 dark:text-gray-400">
                  Keep checkd running in the system tray when the window is closed.
                </span>
              </span>
            </label>
          </section>

          <section>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
              Startup
            </h3>
            <label className="flex items-start gap-3 text-sm text-gray-700 dark:text-gray-200">
              <input
                type="checkbox"
                checked={autostart}
                disabled={autostartLoading}
                onChange={(e) => handleAutostartChange(e.currentTarget.checked)}
                className="mt-0.5 size-4 accent-blue-600 disabled:opacity-50"
              />
              <span>
                Autostart
                <span className="block text-xs text-gray-500 dark:text-gray-400">
                  Start checkd automatically on system startup.
                </span>
              </span>
            </label>
          </section>

          <section>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
              Data
            </h3>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={handleExport}
                disabled={exporting}
                className="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
              >
                {exporting ? "Exporting…" : "Export data"}
              </button>
              {exportStatus && (
                <span className="min-w-0 truncate text-xs text-gray-500 dark:text-gray-400">
                  {exportStatus}
                </span>
              )}
            </div>
          </section>

          <section>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-gray-500">
              Updates
            </h3>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={handleCheckUpdate}
                disabled={checkingUpdate}
                className="rounded-md border border-gray-300 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
              >
                {checkingUpdate ? "Checking…" : "Check for updates"}
              </button>
              {updateStatus && (
                <span className="min-w-0 truncate text-xs text-gray-500 dark:text-gray-400">
                  {updateStatus}
                </span>
              )}
            </div>
          </section>

          <GoogleSyncSection />
        </div>

        <div className="mt-5 flex justify-end">
          <button
            type="button"
            onClick={onClose}
            className="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
