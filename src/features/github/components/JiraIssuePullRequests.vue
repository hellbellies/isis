<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { usePullRequestsStore } from "../stores/pullRequestsStore";
import { pullRequestStatus } from "./pullRequestStatus";
import type { PullRequest } from "@/types";

/** Compact badges for the pull requests (any state) that mention a Jira issue. */
const props = defineProps<{ issueId: string }>();

/** More than this collapse into a "+N" badge. */
const MAX_SHOWN = 3;

const store = usePullRequestsStore();
const { byJiraIssue } = storeToRefs(store);

const pullRequests = computed(() => byJiraIssue.value[props.issueId] ?? []);
const shown = computed(() => pullRequests.value.slice(0, MAX_SHOWN));
const hidden = computed(() => pullRequests.value.slice(MAX_SHOWN));

function describe(pr: PullRequest): string {
  return `${pr.repository}#${pr.number} · ${pullRequestStatus(pr).label} · ${pr.title}`;
}
</script>

<template>
  <div v-if="pullRequests.length" class="flex shrink-0 items-center gap-1">
    <UTooltip v-for="pr in shown" :key="pr.id" :text="describe(pr)">
      <UButton
        size="xs"
        variant="subtle"
        :color="pullRequestStatus(pr).color"
        :icon="pullRequestStatus(pr).icon"
        :label="`#${pr.number}`"
        :aria-label="`Open ${describe(pr)} on GitHub`"
        @click="store.open(pr.id)"
      />
    </UTooltip>
    <UTooltip v-if="hidden.length" :text="hidden.map(describe).join(' / ')">
      <UBadge color="neutral" variant="subtle" size="sm" :label="`+${hidden.length}`" />
    </UTooltip>
  </div>
</template>
