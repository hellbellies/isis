import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, JiraSettings, JiraSettingsInput, JiraUser } from "@/types";

export const useJiraStore = defineStore("jira", () => {
  const settings = ref<JiraSettings | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<AppError | null>(null);

  // Result of the last connection test; cleared whenever the form changes.
  const testing = ref(false);
  const testedUser = ref<JiraUser | null>(null);
  const testError = ref<AppError | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await call("get_jira_settings");
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loading.value = false;
    }
  }

  async function save(input: JiraSettingsInput): Promise<boolean> {
    saving.value = true;
    error.value = null;
    try {
      settings.value = await call("update_jira_settings", { input });
      return true;
    } catch (err: unknown) {
      error.value = err as AppError;
      return false;
    } finally {
      saving.value = false;
    }
  }

  async function clearToken(): Promise<void> {
    error.value = null;
    try {
      settings.value = await call("clear_jira_token");
    } catch (err: unknown) {
      error.value = err as AppError;
    }
  }

  async function testConnection(input: JiraSettingsInput): Promise<void> {
    testing.value = true;
    resetTest();
    try {
      testedUser.value = await call("test_jira_connection", { input });
    } catch (err: unknown) {
      testError.value = err as AppError;
    } finally {
      testing.value = false;
    }
  }

  function resetTest(): void {
    testedUser.value = null;
    testError.value = null;
  }

  return {
    settings,
    loading,
    saving,
    error,
    testing,
    testedUser,
    testError,
    load,
    save,
    clearToken,
    testConnection,
    resetTest,
  };
});
