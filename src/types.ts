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
   * Weekday to recur on for weekly rules (0 = Sunday … 6 = Saturday);
   * null means "same weekday as start". Ignored for non-weekly units.
   */
  weekday: number | null;
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
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};
