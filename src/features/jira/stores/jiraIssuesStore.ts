import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { call } from "@/lib/ipc";
import type {
  AppError,
  JiraIssue,
  JiraProjectStatus,
  JiraStatusRef,
  JiraSyncSummary,
} from "@/types";

/** Stable string id for a (project, status) pair, e.g. `FOO:3`. */
export function statusRefId(status: JiraStatusRef): string {
  return `${status.projectKey}:${status.statusId}`;
}

export const useJiraIssuesStore = defineStore("jiraIssues", () => {
  const issues = ref<JiraIssue[]>([]);
  const lastSyncedAt = ref<number | null>(null);
  const statusOptions = ref<JiraProjectStatus[]>([]);
  const selectedStatuses = ref<JiraStatusRef[]>([]);
  const loaded = ref(false);
  const syncing = ref(false);
  const error = ref<AppError | null>(null);

  /** Issues matching the status filter; all issues when nothing is selected. */
  const visibleIssues = computed(() => {
    if (selectedStatuses.value.length === 0) return issues.value;
    const selected = new Set(selectedStatuses.value.map(statusRefId));
    return issues.value.filter((issue) =>
      selected.has(statusRefId({ projectKey: issue.projectKey, statusId: issue.statusId })),
    );
  });

  /** Reads the locally stored issues and status filter; no network access. */
  async function load(): Promise<void> {
    error.value = null;
    try {
      const [list, filter] = await Promise.all([
        call("list_jira_issues"),
        call("get_jira_status_filter"),
      ]);
      issues.value = list.issues;
      lastSyncedAt.value = list.lastSyncedAt;
      statusOptions.value = filter.options;
      selectedStatuses.value = filter.selected;
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loaded.value = true;
    }
  }

  /** Fetches changes from Jira (or everything when `full`), then reloads the local data. */
  async function sync(full = false): Promise<JiraSyncSummary | null> {
    syncing.value = true;
    error.value = null;
    try {
      const summary = await call("sync_jira_issues", { full });
      await load();
      return summary;
    } catch (err: unknown) {
      error.value = err as AppError;
      return null;
    } finally {
      syncing.value = false;
    }
  }

  /** Persists the status selection. Optimistic; rolls back if Rust rejects. */
  async function setStatusFilter(selected: JiraStatusRef[]): Promise<void> {
    const previous = selectedStatuses.value;
    selectedStatuses.value = selected;
    try {
      selectedStatuses.value = (await call("set_jira_status_filter", { selected })).selected;
    } catch (err: unknown) {
      selectedStatuses.value = previous;
      error.value = err as AppError;
    }
  }

  async function open(key: string): Promise<void> {
    try {
      await call("open_jira_issue", { key });
    } catch (err: unknown) {
      error.value = err as AppError;
    }
  }

  return {
    issues,
    visibleIssues,
    lastSyncedAt,
    statusOptions,
    selectedStatuses,
    loaded,
    syncing,
    error,
    load,
    sync,
    setStatusFilter,
    open,
  };
});
