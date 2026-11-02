/** Mirrors `Todo` in `src-tauri/src/models/todos.rs`. */
export interface Todo {
  id: number;
  title: string;
  /** Plain text; may be empty. */
  description: string;
  /** Unix epoch milliseconds. */
  createdAt: number;
  updatedAt: number;
  /** When the todo was completed; null while open. */
  completedAt: number | null;
  workstreamId: number | null;
}

/** Mirrors `TodoInput` in `src-tauri/src/models/todos.rs`. */
export interface TodoInput {
  title: string;
  description: string;
}
