import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, Settings } from "@/types";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings | null>(null);
  const loading = ref(false);
  const error = ref<AppError | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await call("get_settings");
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loading.value = false;
    }
  }

  async function update(patch: Partial<Settings>): Promise<void> {
    if (!settings.value) return;
    const previous = settings.value;
    // Optimistic update; roll back if Rust rejects.
    settings.value = { ...previous, ...patch };
    error.value = null;
    try {
      settings.value = await call("update_settings", { settings: settings.value });
    } catch (err: unknown) {
      settings.value = previous;
      error.value = err as AppError;
    }
  }

  return { settings, loading, error, load, update };
});
