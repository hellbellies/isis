<script setup lang="ts">
import { computed, onMounted, reactive, watch } from "vue";
import { storeToRefs } from "pinia";
import { useGitHubStore } from "../stores/githubStore";

const store = useGitHubStore();
const { settings, loading, saving, error, testing, testedUser, testError } = storeToRefs(store);

const form = reactive({ repositories: "", token: "" });

// Sync the form from what Rust stored (normalized entries).
watch(
  settings,
  (value) => {
    if (value) form.repositories = value.repositories.join(", ");
  },
  { immediate: true },
);

// A stale test result would be misleading once the inputs change.
watch(form, () => store.resetTest());

/** A blank field means "use the token already in the keychain". */
const enteredToken = computed(() => (form.token.trim() ? form.token : null));

async function onSubmit(): Promise<void> {
  const saved = await store.save({
    repositories: form.repositories.split(","),
    token: enteredToken.value,
  });
  if (saved) form.token = "";
}

onMounted(() => {
  if (!settings.value) void store.load();
});
</script>

<template>
  <UCard>
    <template #header>
      <h2 class="font-semibold text-highlighted">GitHub</h2>
      <p class="text-sm text-muted">The token is stored in the system keychain.</p>
    </template>

    <div v-if="loading" class="space-y-4">
      <USkeleton class="h-8 w-full" />
      <USkeleton class="h-8 w-full" />
    </div>

    <form v-else-if="settings" class="space-y-4" @submit.prevent="onSubmit">
      <UFormField label="Personal access token" name="token">
        <template #description>
          <span v-if="settings?.hasToken" class="inline-flex items-center gap-1">
            <UIcon name="i-lucide-shield-check" class="text-success" />
            A token is saved. Leave blank to keep it.
          </span>
          <span v-else>
            Classic token with the <code>repo</code> scope, or a fine-grained token with
            read access to pull requests.
          </span>
        </template>
        <UInput
          v-model="form.token"
          type="password"
          autocomplete="off"
          :placeholder="settings?.hasToken ? '••••••••••••' : 'ghp_… or github_pat_…'"
          icon="i-lucide-key-round"
          class="w-full"
        />
      </UFormField>

      <UFormField
        label="Repositories (optional)"
        name="repositories"
        description="Comma-separated owner/repo or owner entries to limit the search. Leave empty for all repositories."
      >
        <UInput
          v-model="form.repositories"
          placeholder="acme/app, acme-org"
          icon="i-lucide-folder-git-2"
          class="w-full"
        />
      </UFormField>

      <UAlert
        v-if="testedUser"
        color="success"
        variant="subtle"
        icon="i-lucide-circle-check"
        :title="`Connected as ${testedUser.login}`"
        :description="testedUser.name ?? undefined"
      />
      <UAlert
        v-else-if="testError"
        color="error"
        variant="subtle"
        icon="i-lucide-plug-zap"
        title="Connection failed"
        :description="testError.message"
      />

      <div class="flex justify-end gap-2">
        <UButton
          v-if="settings?.hasToken"
          color="error"
          variant="ghost"
          icon="i-lucide-trash-2"
          label="Remove token"
          @click="store.clearToken()"
        />
        <UButton
          color="neutral"
          variant="outline"
          icon="i-lucide-plug"
          label="Test connection"
          :loading="testing"
          @click="store.testConnection(enteredToken)"
        />
        <UButton type="submit" icon="i-lucide-save" label="Save" :loading="saving" />
      </div>
    </form>

    <UAlert
      v-if="error"
      class="mt-4"
      color="error"
      variant="subtle"
      icon="i-lucide-circle-alert"
      :title="error.message"
    />
  </UCard>
</template>
