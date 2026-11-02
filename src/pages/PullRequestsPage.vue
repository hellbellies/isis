<script setup lang="ts">
import { PullRequestList, usePullRequestsStore } from "@/features/github";
import { WorkstreamSelect } from "@/features/workstreams";

const pullRequestsStore = usePullRequestsStore();
</script>

<template>
  <UDashboardPanel id="pull-requests">
    <template #header>
      <UDashboardNavbar title="Pull requests" icon="i-lucide-git-pull-request" />
    </template>
    <template #body>
      <PullRequestList>
        <template #actions="{ pullRequest }">
          <WorkstreamSelect
            :item="{ itemType: 'pullRequest', itemId: pullRequest.id }"
            :workstream-id="pullRequest.workstreamId"
            @changed="pullRequestsStore.applyWorkstream(pullRequest.id, $event)"
          />
        </template>
      </PullRequestList>
    </template>
  </UDashboardPanel>
</template>
