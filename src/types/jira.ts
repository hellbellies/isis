/** Mirrors `JiraSettings` in `src-tauri/src/models/jira.rs`. The token itself never reaches the frontend. */
export interface JiraSettings {
  baseUrl: string;
  email: string;
  hasToken: boolean;
}

/** Mirrors `JiraSettingsInput` in `src-tauri/src/models/jira.rs`. Omit `apiToken` to keep the stored one. */
export interface JiraSettingsInput {
  baseUrl: string;
  email: string;
  apiToken?: string | null;
}

/** Mirrors `JiraUser` in `src-tauri/src/models/jira.rs`. */
export interface JiraUser {
  displayName: string;
  emailAddress?: string | null;
}
