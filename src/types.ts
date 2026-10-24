export type RepeatUnit = "day" | "week" | "month" | "year";

export type RepeatEnd =
  | { type: "never" }
  | { type: "on"; date: string }
  | { type: "after"; count: number };

export type MonthRepeat =
  | { type: "day"; day: number }
  | { type: "last-day" }
  | { type: "weekday"; weekday: number; week: 1 | 2 | 3 | 4 | "last" };

export type Repeat = {
  unit: RepeatUnit;
  interval: number;
  start: string;
  time: string | null;
  weekdays: number[];
  monthRepeat: MonthRepeat | null;
  end: RepeatEnd;
};

export type Task = {
  id: string;
  title: string;
  description: string;
  dueDate: string | null;
  dueTime: string | null;
  repeat: Repeat | null;
  completed: boolean;
  nextOccurrence: string | null;
  occurrenceIndex: number;
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};

export type AppData = {
  lists: List[];
};

export type SyncStatus = {
  configured: boolean;
  connected: boolean;
  account: string | null;
  pending: number;
  lastSyncAt: number | null;
  lastError: string | null;
};

export type CredentialsStatus = {
  configured: boolean;
  clientId: string | null;
  secretHint: string | null;
  source: string | null;
  envFile: string | null;
};
