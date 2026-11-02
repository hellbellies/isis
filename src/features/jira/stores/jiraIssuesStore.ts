import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, JiraIssue, JiraSyncSummary } from "@/types";

export const useJiraIssuesStore = defineStore("jiraIssues", () => {
  const issues = ref<JiraIssue[]>([]);
  const lastSyncedAt = ref<number | null>(null);
  const loaded = ref(false);
  const syncing = ref(false);
  const error = ref<AppError | null>(null);

  /** Reads the locally stored issues; no network access. */
  async function load(): Promise<void> {
    error.value = null;
    try {
      const list = await call("list_jira_issues");
      issues.value = list.issues;
      lastSyncedAt.value = list.lastSyncedAt;
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loaded.value = true;
    }
  }

  /** Fetches changes from Jira (or everything when `full`), then reloads the local list. */
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

  async function open(key: string): Promise<void> {
    try {
      await call("open_jira_issue", { key });
    } catch (err: unknown) {
      error.value = err as AppError;
    }
  }

  return { issues, lastSyncedAt, loaded, syncing, error, load, sync, open };
});
