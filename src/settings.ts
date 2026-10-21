export type Theme = "system" | "light" | "dark";

export type Settings = {
  theme: Theme;
  /** Keep the app running in the system tray when the window is closed. */
  closeToTray: boolean;
};

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  closeToTray: false,
};

const STORAGE_KEY = "checkd.settings";

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    const parsed = JSON.parse(raw) as Partial<Settings>;
    return {
      theme: parsed.theme ?? DEFAULT_SETTINGS.theme,
      closeToTray: parsed.closeToTray ?? DEFAULT_SETTINGS.closeToTray,
    };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: Settings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Storage unavailable (e.g. private mode) — settings just won't persist.
  }
}

function prefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Toggles the `dark` class on <html>, which drives Tailwind's `dark:` variants. */
export function applyTheme(theme: Theme) {
  const dark = theme === "dark" || (theme === "system" && prefersDark());
  document.documentElement.classList.toggle("dark", dark);
}

/** Re-applies the theme when the OS colour scheme changes (system theme only). */
export function watchSystemTheme(onChange: () => void): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", onChange);
  return () => mq.removeEventListener("change", onChange);
}
