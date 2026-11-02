<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useRoute, useRouter } from "vue-router";
import { WorkstreamOverview, useWorkstreamsStore } from "@/features/workstreams";

const route = useRoute();
const router = useRouter();
const { active, completed } = storeToRefs(useWorkstreamsStore());

const createOpen = ref(false);

/** Kept in the URL (`?status=completed`) so it survives navigating into a workstream and back. */
const status = computed<"active" | "completed">({
  get: () => (route.query.status === "completed" ? "completed" : "active"),
  set: (value) =>
    void router.replace({ query: value === "completed" ? { status: "completed" } : {} }),
});

const tabs = computed(() => [
  { label: "Active", value: "active", icon: "i-lucide-layers", badge: active.value.length },
  {
    label: "Completed",
    value: "completed",
    icon: "i-lucide-circle-check",
    badge: completed.value.length,
  },
]);
</script>

<template>
  <UDashboardPanel id="workstreams">
    <template #header>
      <UDashboardNavbar title="Workstreams">
        <template #right>
          <UButton icon="i-lucide-plus" label="New workstream" @click="createOpen = true" />
        </template>
      </UDashboardNavbar>
      <UDashboardToolbar>
        <UTabs
          :model-value="status"
          :items="tabs"
          :content="false"
          variant="link"
          @update:model-value="status = $event === 'completed' ? 'completed' : 'active'"
        />
      </UDashboardToolbar>
    </template>
    <template #body>
      <WorkstreamOverview v-model:create-open="createOpen" :status="status" />
    </template>
  </UDashboardPanel>
</template>
