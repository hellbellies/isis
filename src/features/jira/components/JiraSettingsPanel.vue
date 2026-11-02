<script setup lang="ts">
import { onMounted, reactive, watch } from "vue";
import { storeToRefs } from "pinia";
import { useJiraStore } from "../stores/jiraStore";
import type { JiraSettingsInput } from "@/types";

const store = useJiraStore();
const { settings, loading, saving, error, testing, testedUser, testError } = storeToRefs(store);

const form = reactive({ baseUrl: "", email: "", apiToken: "" });

// Sync the form from what Rust stored (e.g. the normalized URL).
watch(
  settings,
  (value) => {
    if (!value) return;
    form.baseUrl = value.baseUrl;
    form.email = value.email;
  },
  { immediate: true },
);

// A stale test result would be misleading once the inputs change.
watch(form, () => store.resetTest());

function toInput(): JiraSettingsInput {
  return {
    baseUrl: form.baseUrl,
    email: form.email,
    // A blank field means "use the token already in the keychain".
    apiToken: form.apiToken.trim() ? form.apiToken : null,
  };
}

async function onSubmit(): Promise<void> {
  const saved = await store.save(toInput());
  if (saved) form.apiToken = "";
}

onMounted(() => {
  if (!settings.value) void store.load();
});
</script>

<template>
  <UCard>
    <template #header>
      <h2 class="font-semibold text-highlighted">Jira</h2>
      <p class="text-sm text-muted">The API token is stored in the system keychain.</p>
    </template>

    <div v-if="loading" class="space-y-4">
      <USkeleton class="h-8 w-full" />
      <USkeleton class="h-8 w-full" />
      <USkeleton class="h-8 w-full" />
    </div>

    <form v-else-if="settings" class="space-y-4" @submit.prevent="onSubmit">
      <UFormField label="Site URL" name="baseUrl">
        <UInput
          v-model="form.baseUrl"
          type="url"
          placeholder="https://your-team.atlassian.net"
          icon="i-lucide-link"
          class="w-full"
        />
      </UFormField>

      <UFormField label="Email" name="email" description="The Atlassian account the token belongs to.">
        <UInput
          v-model="form.email"
          type="email"
          placeholder="you@example.com"
          icon="i-lucide-at-sign"
          class="w-full"
        />
      </UFormField>

      <UFormField label="API token" name="apiToken">
        <template #description>
          <span v-if="settings?.hasToken" class="inline-flex items-center gap-1">
            <UIcon name="i-lucide-shield-check" class="text-success" />
            A token is saved. Leave blank to keep it.
          </span>
          <span v-else>No token saved yet.</span>
        </template>
        <UInput
          v-model="form.apiToken"
          type="password"
          autocomplete="off"
          :placeholder="settings?.hasToken ? '••••••••••••' : 'Paste your API token'"
          icon="i-lucide-key-round"
          class="w-full"
        />
      </UFormField>

      <UAlert
        v-if="testedUser"
        color="success"
        variant="subtle"
        icon="i-lucide-circle-check"
        :title="`Connected as ${testedUser.displayName}`"
        :description="testedUser.emailAddress ?? undefined"
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
          @click="store.testConnection(toInput())"
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
