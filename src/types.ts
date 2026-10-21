export type Task = {
  id: string;
  title: string;
};

export type List = {
  id: string;
  name: string;
  color: string;
  tasks: Task[];
};
