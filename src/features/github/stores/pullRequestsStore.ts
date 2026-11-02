import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, GitHubSyncSummary, PullRequest } from "@/types";

/** Cached pull requests that involve the user. */
export const usePullRequestsStore = defineStore("pullRequests", () => {
  const pullRequests = ref<PullRequest[]>([]);
  const lastSyncedAt = ref<number | null>(null);
  const loaded = ref(false);
  const syncing = ref(false);
  const error = ref<AppError | null>(null);

  /** Reads the locally stored pull requests; no network access. */
  async function load(): Promise<void> {
    error.value = null;
    try {
      const list = await call("list_pull_requests");
      pullRequests.value = list.pullRequests;
      lastSyncedAt.value = list.lastSyncedAt;
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loaded.value = true;
    }
  }

  /** Fetches from GitHub, then reloads the local list. */
  async function sync(): Promise<GitHubSyncSummary | null> {
    syncing.value = true;
    error.value = null;
    try {
      const summary = await call("sync_pull_requests");
      await load();
      return summary;
    } catch (err: unknown) {
      error.value = err as AppError;
      return null;
    } finally {
      syncing.value = false;
    }
  }

  /** Reflects a saved workstream change locally, without reloading. */
  function applyWorkstream(id: string, workstreamId: number | null): void {
    const pullRequest = pullRequests.value.find((candidate) => candidate.id === id);
    if (pullRequest) pullRequest.workstreamId = workstreamId;
  }

  async function open(id: string): Promise<void> {
    try {
      await call("open_pull_request", { id });
    } catch (err: unknown) {
      error.value = err as AppError;
    }
  }

  return { pullRequests, lastSyncedAt, loaded, syncing, error, load, sync, applyWorkstream, open };
});
