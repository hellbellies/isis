/** Mirrors `JiraSettings` in `src-tauri/src/models/jira.rs`. The token itself never reaches the frontend. */
export interface JiraSettings {
  baseUrl: string;
  email: string;
  projectKeys: string[];
  hasToken: boolean;
}

/** Mirrors `JiraSettingsInput` in `src-tauri/src/models/jira.rs`. Omit `apiToken` to keep the stored one. */
export interface JiraSettingsInput {
  baseUrl: string;
  email: string;
  projectKeys: string[];
  apiToken?: string | null;
}

/** Mirrors `JiraUser` in `src-tauri/src/models/jira.rs`. */
export interface JiraUser {
  displayName: string;
  emailAddress?: string | null;
}

/** Mirrors `JiraIssue` in `src-tauri/src/models/jira.rs`. */
export interface JiraIssue {
  id: string;
  key: string;
  summary: string;
  /** ISO-8601 timestamp of the last change in Jira. */
  updated: string;
}

/** Mirrors `JiraIssueList` in `src-tauri/src/models/jira.rs`. */
export interface JiraIssueList {
  issues: JiraIssue[];
  /** Unix epoch milliseconds of the last successful sync. */
  lastSyncedAt: number | null;
}

/** Mirrors `JiraSyncMode` in `src-tauri/src/models/jira.rs`. */
export type JiraSyncMode = "full" | "delta";

/** Mirrors `JiraSyncSummary` in `src-tauri/src/models/jira.rs`. */
export interface JiraSyncSummary {
  mode: JiraSyncMode;
  upserted: number;
  removed: number;
}
