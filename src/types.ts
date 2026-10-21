export type Task = {
  id: string;
  title: string;
  /** Due date as YYYY-MM-DD, or null when unset. */
  dueDate: string | null;
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};
