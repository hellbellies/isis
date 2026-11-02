<script setup lang="ts">
import { computed, ref } from "vue";
import { useToast } from "@nuxt/ui/composables";
import { useTodosStore } from "../stores/todosStore";
import type { AppError, Todo } from "@/types";

const props = defineProps<{ todo: Todo }>();

const emit = defineEmits<{
  /** The title was clicked. */
  edit: [todo: Todo];
  /** The todo was completed or reopened. */
  changed: [todo: Todo];
}>();

const store = useTodosStore();
const toast = useToast();
const toggling = ref(false);

const done = computed(() => props.todo.completedAt !== null);
const firstLine = computed(() => props.todo.description.split("\n", 1)[0]);

async function toggle(): Promise<void> {
  toggling.value = true;
  try {
    emit("changed", await store.setCompleted(props.todo.id, !done.value));
  } catch (err: unknown) {
    toast.add({ title: (err as AppError).message, color: "error", icon: "i-lucide-circle-alert" });
  } finally {
    toggling.value = false;
  }
}
</script>

<template>
  <li class="flex items-center gap-3 px-4 py-2.5">
    <UCheckbox
      :model-value="done"
      :disabled="toggling"
      :aria-label="done ? `Reopen ${todo.title}` : `Complete ${todo.title}`"
      @update:model-value="toggle"
    />
    <div class="min-w-0 flex-1">
      <ULink
        as="button"
        class="block max-w-full truncate text-left"
        :class="done ? 'text-muted line-through' : ''"
        title="Edit todo"
        @click="emit('edit', todo)"
      >
        {{ todo.title }}
      </ULink>
      <p v-if="firstLine" class="truncate text-xs text-muted">{{ firstLine }}</p>
    </div>
    <slot name="actions" />
  </li>
</template>
