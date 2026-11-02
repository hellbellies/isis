<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useToast } from "@nuxt/ui/composables";
import { useTimeAgo } from "@vueuse/core";
import { useJiraIssuesStore } from "../stores/jiraIssuesStore";
import type { JiraSyncSummary } from "@/types";

const store = useJiraIssuesStore();
const { issues, lastSyncedAt, loaded, syncing, error } = storeToRefs(store);
const toast = useToast();

const lastSyncedAgo = useTimeAgo(computed(() => lastSyncedAt.value ?? 0));

async function sync(full: boolean): Promise<void> {
  const summary = await store.sync(full);
  if (summary) toast.add({ title: describe(summary), icon: "i-lucide-refresh-cw", color: "success" });
}

function describe(summary: JiraSyncSummary): string {
  if (summary.mode === "full") return `Fetched ${summary.upserted} issues`;
  if (summary.upserted === 0 && summary.removed === 0) return "Already up to date";
  return `${summary.upserted} updated, ${summary.removed} removed`;
}

onMounted(() => {
  if (!loaded.value) void store.load();
});
</script>

<template>
  <UCard :ui="{ body: 'p-0 sm:p-0' }">
    <template #header>
      <div class="flex items-center justify-between gap-4">
        <div>
          <h2 class="font-semibold text-highlighted">My Jira issues</h2>
          <p class="text-sm text-muted">
            <template v-if="lastSyncedAt">Last fetched {{ lastSyncedAgo }}</template>
            <template v-else>Not fetched yet</template>
          </p>
        </div>
        <div class="flex gap-2">
          <UTooltip text="Re-fetch everything, e.g. to drop issues deleted in Jira">
            <UButton
              color="neutral"
              variant="ghost"
              icon="i-lucide-database-zap"
              label="Full refresh"
              :disabled="syncing"
              @click="sync(true)"
            />
          </UTooltip>
          <UButton
            icon="i-lucide-refresh-cw"
            label="Fetch"
            :loading="syncing"
            @click="sync(false)"
          />
        </div>
      </div>
    </template>

    <UAlert
      v-if="error"
      class="m-4"
      color="error"
      variant="subtle"
      icon="i-lucide-circle-alert"
      :title="error.message"
    />

    <ul v-if="issues.length" class="divide-y divide-default">
      <li v-for="issue in issues" :key="issue.id" class="flex items-center gap-3 px-4 py-2.5">
        <UBadge color="neutral" variant="subtle" class="shrink-0 font-mono" :label="issue.key" />
        <ULink
          as="button"
          class="truncate text-left"
          :title="`Open ${issue.key} in Jira`"
          @click="store.open(issue.key)"
        >
          {{ issue.summary }}
        </ULink>
      </li>
    </ul>

    <UEmpty
      v-else-if="loaded && !syncing"
      class="py-10"
      icon="i-lucide-inbox"
      title="No issues"
      :description="
        lastSyncedAt
          ? 'Nothing in your projects is assigned to you.'
          : 'Add project keys in the settings, then fetch your issues.'
      "
    />
  </UCard>
</template>
