/** Mirrors `GitHubSettings` in `src-tauri/src/models/github.rs`. The token never reaches the frontend. */
export interface GitHubSettings {
  /** Optional search scope: `owner/repo` or `owner` entries. Empty means all repositories. */
  repositories: string[];
  hasToken: boolean;
}

/** Mirrors `GitHubSettingsInput` in `src-tauri/src/models/github.rs`. Omit `token` to keep the stored one. */
export interface GitHubSettingsInput {
  repositories: string[];
  token?: string | null;
}

/** Mirrors `GitHubUser` in `src-tauri/src/models/github.rs`. */
export interface GitHubUser {
  login: string;
  name: string | null;
}

/** Mirrors `PullRequestState` in `src-tauri/src/models/github.rs`. */
export type PullRequestState = "open" | "closed" | "merged";

/** Mirrors `ReviewDecision` in `src-tauri/src/models/github.rs`. */
export type ReviewDecision = "approved" | "changes_requested" | "review_required";

/** Mirrors `PullRequest` in `src-tauri/src/models/github.rs`. */
export interface PullRequest {
  /** GitHub's GraphQL node id. */
  id: string;
  number: number;
  title: string;
  url: string;
  /** `owner/name`. */
  repository: string;
  author: string;
  state: PullRequestState;
  isDraft: boolean;
  reviewDecision: ReviewDecision | null;
  /** ISO-8601 timestamp of the last change. */
  updatedAt: string;
  authoredByMe: boolean;
  reviewRequested: boolean;
  /** Open in a watched repository, without involving the user. */
  otherOpen: boolean;
  workstreamId: number | null;
}

/** Mirrors `PullRequestList` in `src-tauri/src/models/github.rs`. */
export interface PullRequestList {
  pullRequests: PullRequest[];
  /** Unix epoch milliseconds of the last successful sync. */
  lastSyncedAt: number | null;
}

/** Mirrors `JiraIssuePullRequest` in `src-tauri/src/models/github.rs`. */
export interface JiraIssuePullRequest {
  /** Jira's issue id (not the key). */
  issueId: string;
  pullRequest: PullRequest;
}

/** Mirrors `GitHubSyncSummary` in `src-tauri/src/models/github.rs`. */
export interface GitHubSyncSummary {
  fetched: number;
  removed: number;
  /** Other open pull requests in the watched repositories. */
  others: number;
}
