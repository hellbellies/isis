import type { JiraIssue } from "./jira";

/** Mirrors `Workstream` in `src-tauri/src/models/workstreams.rs`. */
export interface Workstream {
  id: number;
  name: string;
  /** Unix epoch milliseconds. */
  createdAt: number;
  /** Number of linked items of all types. */
  itemCount: number;
}

/** Mirrors `WorkstreamDetail` in `src-tauri/src/models/workstreams.rs`. */
export interface WorkstreamDetail {
  workstream: Workstream;
  jiraIssues: JiraIssue[];
}

/** Mirrors `WorkstreamItemType` in `src-tauri/src/models/workstreams.rs`. */
export type WorkstreamItemType = "jiraIssue";

/** Mirrors `WorkstreamItemRef` in `src-tauri/src/models/workstreams.rs`. */
export interface WorkstreamItemRef {
  itemType: WorkstreamItemType;
  /** The item's id in its own source, e.g. the Jira issue id. */
  itemId: string;
}
