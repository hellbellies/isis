<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useRoute, useRouter } from "vue-router";
import { TodoList, useTodosStore } from "@/features/todos";
import { WorkstreamSelect } from "@/features/workstreams";

const route = useRoute();
const router = useRouter();
const todosStore = useTodosStore();
const { open, completed } = storeToRefs(todosStore);

const createOpen = ref(false);

/** Kept in the URL (`?status=completed`) like the workstreams overview. */
const status = computed<"open" | "completed">({
  get: () => (route.query.status === "completed" ? "completed" : "open"),
  set: (value) =>
    void router.replace({ query: value === "completed" ? { status: "completed" } : {} }),
});

const tabs = computed(() => [
  { label: "Open", value: "open", icon: "i-lucide-list-todo", badge: open.value.length },
  {
    label: "Completed",
    value: "completed",
    icon: "i-lucide-list-checks",
    badge: completed.value.length,
  },
]);
</script>

<template>
  <UDashboardPanel id="todos">
    <template #header>
      <UDashboardNavbar title="Todos" icon="i-lucide-list-todo">
        <template #right>
          <UButton icon="i-lucide-plus" label="New todo" @click="createOpen = true" />
        </template>
      </UDashboardNavbar>
      <UDashboardToolbar>
        <UTabs
          :model-value="status"
          :items="tabs"
          :content="false"
          variant="link"
          @update:model-value="status = $event === 'completed' ? 'completed' : 'open'"
        />
      </UDashboardToolbar>
    </template>
    <template #body>
      <TodoList v-model:create-open="createOpen" :status="status">
        <template #actions="{ todo }">
          <WorkstreamSelect
            :item="{ itemType: 'todo', itemId: String(todo.id) }"
            :workstream-id="todo.workstreamId"
            @changed="todosStore.applyWorkstream(todo.id, $event)"
          />
        </template>
      </TodoList>
    </template>
  </UDashboardPanel>
</template>
