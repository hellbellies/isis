<script setup lang="ts">
import { computed } from "vue";
import { SettingsPanel, useSettingsStore, useThemeSync } from "@/features/settings";

const settingsStore = useSettingsStore();
useThemeSync(computed(() => settingsStore.settings?.theme));
</script>

<template>
  <UApp>
    <main class="mx-auto flex min-h-screen max-w-xl flex-col gap-6 p-8">
      <h1 class="text-2xl font-semibold tracking-tight text-highlighted">isis</h1>
      <SettingsPanel />
    </main>
  </UApp>
</template>
