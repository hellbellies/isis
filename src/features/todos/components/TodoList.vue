<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import { useTodosStore } from "../stores/todosStore";
import TodoEditModal from "./TodoEditModal.vue";
import TodoRow from "./TodoRow.vue";
import type { Todo } from "@/types";

defineSlots<{
  /** Extra controls at the end of each row. */
  actions?(props: { todo: Todo }): unknown;
}>();

const props = defineProps<{ status: "open" | "completed" }>();

/** Controls the "New todo" dialog; the page's navbar button opens it too. */
const createOpen = defineModel<boolean>("createOpen", { default: false });

const store = useTodosStore();
const { open, completed, loaded, error } = storeToRefs(store);

const shown = computed(() => (props.status === "completed" ? completed.value : open.value));

const editing = ref<Todo | null>(null);
const editOpen = ref(false);

function edit(todo: Todo): void {
  editing.value = todo;
  editOpen.value = true;
}

// Local data only, so always reload: other views may have changed todos or links.
onMounted(() => void store.load());
</script>

<template>
  <UAlert
    v-if="error"
    class="mb-4"
    color="error"
    variant="subtle"
    icon="i-lucide-circle-alert"
    :title="error.message"
  />

  <UCard v-if="shown.length" :ui="{ body: 'p-0 sm:p-0' }">
    <ul class="divide-y divide-default">
      <TodoRow v-for="todo in shown" :key="todo.id" :todo="todo" @edit="edit">
        <template #actions>
          <slot name="actions" :todo="todo" />
        </template>
      </TodoRow>
    </ul>
  </UCard>

  <UEmpty
    v-else-if="loaded && status === 'completed'"
    class="py-16"
    icon="i-lucide-list-checks"
    title="No completed todos"
    description="Tick a todo's checkbox to complete it."
  />
  <UEmpty
    v-else-if="loaded"
    class="py-16"
    icon="i-lucide-list-todo"
    :title="completed.length ? 'All done' : 'No todos yet'"
    description="Todos are your own tasks. Add them here or directly in a workstream."
    :actions="[{ label: 'New todo', icon: 'i-lucide-plus', onClick: () => (createOpen = true) }]"
  />

  <TodoEditModal v-model:open="createOpen" />
  <TodoEditModal v-model:open="editOpen" :todo="editing" />
</template>
