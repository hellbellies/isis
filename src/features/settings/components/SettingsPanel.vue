<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useSettingsStore } from "../stores/settingsStore";
import type { Theme } from "@/types";

const store = useSettingsStore();
const { settings, loading, error } = storeToRefs(store);

const themeOptions: { value: Theme; label: string; icon: string }[] = [
  { value: "system", label: "System", icon: "i-lucide-monitor" },
  { value: "light", label: "Light", icon: "i-lucide-sun" },
  { value: "dark", label: "Dark", icon: "i-lucide-moon" },
];

onMounted(() => {
  if (!settings.value) void store.load();
});
</script>

<template>
  <UCard>
    <template #header>
      <h2 class="font-semibold text-highlighted">Settings</h2>
      <p class="text-sm text-muted">Stored by Rust in the app config directory.</p>
    </template>

    <div v-if="loading" class="space-y-4">
      <USkeleton class="h-8 w-full" />
      <USkeleton class="h-8 w-full" />
    </div>

    <div v-else-if="settings" class="space-y-5">
      <UFormField label="Theme" orientation="horizontal">
        <USelect
          :model-value="settings.theme"
          :items="themeOptions"
          class="w-36"
          @update:model-value="store.update({ theme: $event })"
        />
      </UFormField>

      <UFormField
        label="Notifications"
        description="Show desktop notifications."
        orientation="horizontal"
      >
        <USwitch
          :model-value="settings.notificationsEnabled"
          @update:model-value="store.update({ notificationsEnabled: $event })"
        />
      </UFormField>
    </div>

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
