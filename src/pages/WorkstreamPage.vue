<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { useRouter } from "vue-router";
import { useToast } from "@nuxt/ui/composables";
import { WorkstreamItems, WorkstreamNameModal, useWorkstreamsStore } from "@/features/workstreams";
import type { AppError } from "@/types";

const props = defineProps<{ id: number }>();

const store = useWorkstreamsStore();
const { current } = storeToRefs(store);
const router = useRouter();
const toast = useToast();

/** Only trust `current` once it matches the route, to avoid flashing the previous workstream. */
const workstream = computed(() =>
  current.value?.workstream.id === props.id ? current.value.workstream : null,
);

const renameOpen = ref(false);
const deleteOpen = ref(false);
const deleting = ref(false);

async function remove(): Promise<void> {
  deleting.value = true;
  try {
    await store.remove(props.id);
    deleteOpen.value = false;
    await router.push({ name: "workstreams" });
  } catch (err: unknown) {
    toast.add({ title: (err as AppError).message, color: "error", icon: "i-lucide-circle-alert" });
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <UDashboardPanel :id="`workstream-${id}`">
    <template #header>
      <UDashboardNavbar :title="workstream?.name ?? 'Workstream'" icon="i-lucide-layers">
        <template #right>
          <template v-if="workstream">
            <UButton
              color="neutral"
              variant="ghost"
              icon="i-lucide-pencil"
              label="Rename"
              @click="renameOpen = true"
            />
            <UButton
              color="error"
              variant="ghost"
              icon="i-lucide-trash-2"
              label="Delete"
              @click="deleteOpen = true"
            />
          </template>
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <WorkstreamItems :id="id" />
    </template>
  </UDashboardPanel>

  <WorkstreamNameModal
    v-model:open="renameOpen"
    title="Rename workstream"
    submit-label="Rename"
    :initial-name="workstream?.name"
    :save="(name) => store.rename(id, name)"
  />

  <UModal
    v-model:open="deleteOpen"
    title="Delete workstream?"
    :description="`“${workstream?.name}” will be deleted. Its items are not deleted, only removed from it.`"
  >
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="ghost" label="Cancel" @click="deleteOpen = false" />
        <UButton color="error" label="Delete" :loading="deleting" @click="remove" />
      </div>
    </template>
  </UModal>
</template>
