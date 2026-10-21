export type Task = {
  id: string;
  title: string;
  /** Due date as YYYY-MM-DD, or null when unset. */
  dueDate: string | null;
  /** Due time as HH:MM (24h); only meaningful while dueDate is set. */
  dueTime: string | null;
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};
