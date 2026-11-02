<script setup lang="ts">
import { ref } from "vue";
import { WorkstreamOverview } from "@/features/workstreams";

const createOpen = ref(false);
</script>

<template>
  <UDashboardPanel id="workstreams">
    <template #header>
      <UDashboardNavbar title="Workstreams">
        <template #right>
          <UButton icon="i-lucide-plus" label="New workstream" @click="createOpen = true" />
        </template>
      </UDashboardNavbar>
    </template>
    <template #body>
      <WorkstreamOverview v-model:create-open="createOpen" />
    </template>
  </UDashboardPanel>
</template>
