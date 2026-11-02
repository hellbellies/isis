<script setup lang="ts">
import { SettingsPanel } from "@/features/settings";
import { JiraSettingsPanel } from "@/features/jira";
import { GitHubSettingsPanel } from "@/features/github";
</script>

<template>
  <UDashboardPanel id="settings">
    <template #header>
      <UDashboardNavbar title="Settings" icon="i-lucide-settings" />
    </template>
    <template #body>
      <div class="mx-auto flex w-full max-w-xl flex-col gap-6">
        <SettingsPanel />
        <JiraSettingsPanel />
        <GitHubSettingsPanel />
      </div>
    </template>
  </UDashboardPanel>
</template>
