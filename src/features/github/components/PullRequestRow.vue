<script setup lang="ts">
import { computed } from "vue";
import { usePullRequestsStore } from "../stores/pullRequestsStore";
import { pullRequestStatus } from "./pullRequestStatus";
import type { PullRequest } from "@/types";

const props = defineProps<{ pullRequest: PullRequest }>();

const store = usePullRequestsStore();

const statusBadge = computed(() => pullRequestStatus(props.pullRequest));

const involvesMe = computed(
  () => props.pullRequest.authoredByMe || props.pullRequest.reviewRequested,
);
</script>

<template>
  <li class="flex items-center gap-3 px-4 py-2.5">
    <UBadge
      color="neutral"
      variant="subtle"
      class="shrink-0 font-mono"
      :label="`#${pullRequest.number}`"
    />
    <div class="min-w-0 flex-1">
      <ULink
        as="button"
        class="block max-w-full truncate text-left"
        :title="`Open ${pullRequest.repository}#${pullRequest.number} on GitHub`"
        @click="store.open(pullRequest.id)"
      >
        {{ pullRequest.title }}
      </ULink>
      <p class="truncate text-xs text-muted">
        {{ pullRequest.repository }}<template v-if="pullRequest.author"> · {{ pullRequest.author }}</template>
      </p>
    </div>
    <UTooltip v-if="pullRequest.reviewRequested" text="Your review is requested">
      <UIcon name="i-lucide-eye" class="size-4 shrink-0 text-warning" />
    </UTooltip>
    <UTooltip
      v-if="!involvesMe && !pullRequest.otherOpen"
      text="No longer needs you (kept for its workstream)"
    >
      <UIcon name="i-lucide-user-x" class="size-4 shrink-0 text-muted" />
    </UTooltip>
    <UBadge
      :color="statusBadge.color"
      variant="subtle"
      class="shrink-0"
      :label="statusBadge.label"
    />
    <slot name="actions" />
  </li>
</template>
