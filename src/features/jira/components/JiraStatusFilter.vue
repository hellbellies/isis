<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import type { SelectMenuItem } from "@nuxt/ui";
import { statusRefId, useJiraIssuesStore } from "../stores/jiraIssuesStore";
import type { JiraProjectStatus } from "@/types";

const store = useJiraIssuesStore();
const { statusOptions, selectedStatuses } = storeToRefs(store);

/** One group per project, headed by its key. Values are `PROJECT:statusId`. */
const items = computed<SelectMenuItem[][]>(() => {
  const groups = new Map<string, SelectMenuItem[]>();
  for (const status of statusOptions.value) {
    let group = groups.get(status.projectKey);
    if (!group) {
      group = [{ type: "label", label: status.projectKey }];
      groups.set(status.projectKey, group);
    }
    group.push({ label: status.name, value: statusRefId(status) });
  }
  return [...groups.values()];
});

const optionsById = computed(
  () => new Map<string, JiraProjectStatus>(statusOptions.value.map((s) => [statusRefId(s), s])),
);

const selectedIds = computed<string[]>({
  get: () => selectedStatuses.value.map(statusRefId),
  set: (ids) => {
    const refs = ids.flatMap((id) => {
      const status = optionsById.value.get(id);
      return status ? [{ projectKey: status.projectKey, statusId: status.statusId }] : [];
    });
    void store.setStatusFilter(refs);
  },
});

const triggerLabel = computed(() => {
  const count = selectedStatuses.value.length;
  if (count === 0) return "All statuses";
  const [first] = selectedStatuses.value;
  const only = count === 1 && first ? optionsById.value.get(statusRefId(first)) : undefined;
  return only ? `${only.projectKey}: ${only.name}` : `${count} statuses`;
});
</script>

<template>
  <USelectMenu
    v-model="selectedIds"
    :items="items"
    value-key="value"
    multiple
    :clear="selectedIds.length > 0"
    :disabled="statusOptions.length === 0"
    :search-input="{ placeholder: 'Filter statuses…' }"
    icon="i-lucide-list-filter"
    class="w-56"
  >
    <template #default>
      <span :class="selectedIds.length ? 'text-highlighted' : 'text-muted'">{{ triggerLabel }}</span>
    </template>
  </USelectMenu>
</template>
