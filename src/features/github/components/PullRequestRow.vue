<script setup lang="ts">
import { computed } from "vue";
import { usePullRequestsStore } from "../stores/pullRequestsStore";
import type { PullRequest } from "@/types";

const props = defineProps<{ pullRequest: PullRequest }>();

const store = usePullRequestsStore();

type BadgeColor = "neutral" | "info" | "success" | "warning" | "error" | "primary";

/** Merged/closed first, then draft, then the review decision of an open PR. */
const statusBadge = computed<{ label: string; color: BadgeColor }>(() => {
  const pr = props.pullRequest;
  if (pr.state === "merged") return { label: "Merged", color: "primary" };
  if (pr.state === "closed") return { label: "Closed", color: "error" };
  if (pr.isDraft) return { label: "Draft", color: "neutral" };
  switch (pr.reviewDecision) {
    case "approved":
      return { label: "Approved", color: "success" };
    case "changes_requested":
      return { label: "Changes requested", color: "warning" };
    default:
      return { label: "In review", color: "info" };
  }
});

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
