<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { SettingsPanel, useSettingsStore, useThemeSync } from "@/features/settings";
import { JiraIssueList, JiraSettingsPanel } from "@/features/jira";

const settingsStore = useSettingsStore();
useThemeSync(computed(() => settingsStore.settings?.theme));

const settingsOpen = ref(false);

// The settings panels only mount when the slideover opens, but the theme is needed right away.
onMounted(() => {
  if (!settingsStore.settings) void settingsStore.load();
});
</script>

<template>
  <UApp>
    <main class="mx-auto flex min-h-screen max-w-3xl flex-col gap-6 p-8">
      <header class="flex items-center justify-between">
        <h1 class="text-2xl font-semibold tracking-tight text-highlighted">isis</h1>
        <USlideover v-model:open="settingsOpen" title="Settings">
          <UButton
            color="neutral"
            variant="ghost"
            icon="i-lucide-settings"
            aria-label="Settings"
          />
          <template #body>
            <div class="flex flex-col gap-6">
              <SettingsPanel />
              <JiraSettingsPanel />
            </div>
          </template>
        </USlideover>
      </header>
      <JiraIssueList />
    </main>
  </UApp>
</template>
