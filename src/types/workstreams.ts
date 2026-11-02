import type { PullRequest } from "./github";
import type { JiraIssue } from "./jira";
import type { Todo } from "./todos";

/** Mirrors `Workstream` in `src-tauri/src/models/workstreams.rs`. */
export interface Workstream {
  id: number;
  name: string;
  /** Unix epoch milliseconds. */
  createdAt: number;
  /** Unix epoch milliseconds when marked completed; null while active. */
  completedAt: number | null;
  /** Number of linked items of all types. */
  itemCount: number;
}

/** Mirrors `WorkstreamDetail` in `src-tauri/src/models/workstreams.rs`. */
export interface WorkstreamDetail {
  workstream: Workstream;
  jiraIssues: JiraIssue[];
  pullRequests: PullRequest[];
  todos: Todo[];
}

/** Mirrors `WorkstreamItemType` in `src-tauri/src/models/workstreams.rs`. */
export type WorkstreamItemType = "jiraIssue" | "pullRequest" | "todo";

/** Mirrors `WorkstreamItemRef` in `src-tauri/src/models/workstreams.rs`. */
export interface WorkstreamItemRef {
  itemType: WorkstreamItemType;
  /** The item's id in its own source, e.g. the Jira issue id, GitHub node id or todo id. */
  itemId: string;
}
