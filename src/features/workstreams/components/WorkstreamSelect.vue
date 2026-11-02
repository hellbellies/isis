<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useToast } from "@nuxt/ui/composables";
import { useWorkstreamsStore } from "../stores/workstreamsStore";
import type { AppError, WorkstreamItemRef } from "@/types";

const props = defineProps<{
  item: WorkstreamItemRef;
  /** The item's current workstream. */
  workstreamId: number | null;
}>();

/** Emitted after the assignment was saved. */
const emit = defineEmits<{ changed: [workstreamId: number | null] }>();

const store = useWorkstreamsStore();
const { workstreams, loaded } = storeToRefs(store);
const toast = useToast();

/** Active workstreams, plus the item's current one if it was completed (so its name still shows). */
const items = computed(() =>
  workstreams.value
    .filter((w) => w.completedAt === null || w.id === props.workstreamId)
    .map((w) => ({ label: w.name, value: w.id })),
);

async function assign(workstreamId: number | null): Promise<void> {
  if (workstreamId === props.workstreamId) return;
  try {
    await store.assign(props.item, workstreamId);
    emit("changed", workstreamId);
  } catch (err: unknown) {
    toast.add({ title: (err as AppError).message, color: "error", icon: "i-lucide-circle-alert" });
  }
}

async function createAndAssign(name: string): Promise<void> {
  try {
    const workstream = await store.create(name);
    await assign(workstream.id);
  } catch (err: unknown) {
    toast.add({ title: (err as AppError).message, color: "error", icon: "i-lucide-circle-alert" });
  }
}

onMounted(() => {
  if (!loaded.value) void store.load();
});
</script>

<template>
  <USelectMenu
    :model-value="workstreamId ?? undefined"
    :items="items"
    value-key="value"
    create-item
    clear
    size="sm"
    variant="ghost"
    icon="i-lucide-layers"
    placeholder="Add to workstream"
    :search-input="{ placeholder: 'Find or create…' }"
    class="w-48"
    @update:model-value="assign(typeof $event === 'number' ? $event : null)"
    @create="createAndAssign"
  />
</template>
