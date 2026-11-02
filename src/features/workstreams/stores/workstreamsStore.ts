import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, Workstream, WorkstreamDetail, WorkstreamItemRef } from "@/types";

/**
 * Workstreams and the currently opened workstream.
 * Mutations reject with an `AppError` so the calling UI (modal, dropdown, …)
 * can show the message where the user acted.
 */
export const useWorkstreamsStore = defineStore("workstreams", () => {
  const workstreams = ref<Workstream[]>([]);
  const loaded = ref(false);
  const error = ref<AppError | null>(null);

  const current = ref<WorkstreamDetail | null>(null);
  const currentError = ref<AppError | null>(null);

  async function load(): Promise<void> {
    error.value = null;
    try {
      workstreams.value = await call("list_workstreams");
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loaded.value = true;
    }
  }

  async function loadDetail(id: number): Promise<void> {
    currentError.value = null;
    try {
      current.value = await call("get_workstream", { id });
    } catch (err: unknown) {
      current.value = null;
      currentError.value = err as AppError;
    }
  }

  async function create(name: string): Promise<Workstream> {
    const workstream = await call("create_workstream", { name });
    await load();
    return workstream;
  }

  async function rename(id: number, name: string): Promise<Workstream> {
    const workstream = await call("rename_workstream", { id, name });
    if (current.value?.workstream.id === id) current.value.workstream = workstream;
    await load();
    return workstream;
  }

  async function remove(id: number): Promise<void> {
    await call("delete_workstream", { id });
    if (current.value?.workstream.id === id) current.value = null;
    await load();
  }

  /** Moves an item into a workstream, or out of its workstream when `workstreamId` is null. */
  async function assign(item: WorkstreamItemRef, workstreamId: number | null): Promise<void> {
    await call("set_item_workstream", { item, workstreamId });
    const openId = current.value?.workstream.id;
    await Promise.all([load(), openId === undefined ? undefined : loadDetail(openId)]);
  }

  return {
    workstreams,
    loaded,
    error,
    current,
    currentError,
    load,
    loadDetail,
    create,
    rename,
    remove,
    assign,
  };
});
