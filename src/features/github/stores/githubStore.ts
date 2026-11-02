import { defineStore } from "pinia";
import { ref } from "vue";
import { call } from "@/lib/ipc";
import type { AppError, GitHubSettings, GitHubSettingsInput, GitHubUser } from "@/types";

/** GitHub connection settings. */
export const useGitHubStore = defineStore("github", () => {
  const settings = ref<GitHubSettings | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<AppError | null>(null);

  // Result of the last connection test; cleared whenever the form changes.
  const testing = ref(false);
  const testedUser = ref<GitHubUser | null>(null);
  const testError = ref<AppError | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await call("get_github_settings");
    } catch (err: unknown) {
      error.value = err as AppError;
    } finally {
      loading.value = false;
    }
  }

  async function save(input: GitHubSettingsInput): Promise<boolean> {
    saving.value = true;
    error.value = null;
    try {
      settings.value = await call("update_github_settings", { input });
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
      settings.value = await call("clear_github_token");
    } catch (err: unknown) {
      error.value = err as AppError;
    }
  }

  /** Tests `token`, or the stored token when null. */
  async function testConnection(token: string | null): Promise<void> {
    testing.value = true;
    resetTest();
    try {
      testedUser.value = await call("test_github_connection", { token });
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
