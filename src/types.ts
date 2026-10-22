export type RepeatUnit = "day" | "week" | "month" | "year";

export type RepeatEnd =
  | { type: "never" }
  | { type: "on"; date: string }
  | { type: "after"; count: number };

/** How a monthly rule lands within the month. */
export type MonthRepeat =
  | { type: "day"; day: number } // 1..31
  | { type: "last-day" }
  | { type: "weekday"; weekday: number; week: 1 | 2 | 3 | 4 | "last" };

export type Repeat = {
  unit: RepeatUnit;
  /** Repeat every N units. */
  interval: number;
  /** First occurrence as YYYY-MM-DD. */
  start: string;
  /** Time of day for occurrences as HH:MM (24h), or null for no time. */
  time: string | null;
  /**
   * Weekdays to recur on for weekly rules (0 = Sunday … 6 = Saturday).
   * Empty means "same weekday as start". Ignored for non-weekly units.
   */
  weekdays: number[];
  /** Day/anchor within the month; only for monthly rules. Null = start date's day. */
  monthRepeat: MonthRepeat | null;
  end: RepeatEnd;
};

export type Task = {
  id: string;
  title: string;
  description: string;
  /** Due date as YYYY-MM-DD, or null when unset. */
  dueDate: string | null;
  /** Due time as HH:MM (24h); only meaningful while dueDate is set. */
  dueTime: string | null;
  repeat: Repeat | null;
  /** Whether the active occurrence is done. */
  completed: boolean;
  /**
   * Date of the active occurrence of a repeating task (YYYY-MM-DD). The
   * occurrence advances every time the task is completed.
   */
  nextOccurrence: string | null;
  /** How many occurrences of the series have been completed. */
  occurrenceIndex: number;
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};

/** The whole database, as returned by the `load_data` command. */
export type AppData = {
  lists: List[];
};

/** The state of the Google Calendar sync. */
export type SyncStatus = {
  /** The OAuth client is stored — the user can connect. */
  configured: boolean;
  /** A Google account is connected. */
  connected: boolean;
  /** The connected account. */
  account: string | null;
  /** Local changes that are waiting to be pushed. */
  pending: number;
  /** When the last sync ran (unix milliseconds). */
  lastSyncAt: number | null;
  /** The message of the last failed sync. */
  lastError: string | null;
};

/**
 * The OAuth client the app found in the environment — the secret itself never
 * leaves the environment, only a hint that shows it is there.
 */
export type CredentialsStatus = {
  configured: boolean;
  clientId: string | null;
  secretHint: string | null;
  /** Where the client came from — an environment variable or a `.env` file. */
  source: string | null;
  /** The file the app reads when the environment says nothing. */
  envFile: string | null;
};
