<script setup lang="ts">
import { onMounted } from "vue";
import { storeToRefs } from "pinia";
import { useRouter } from "vue-router";
import { useWorkstreamsStore } from "../stores/workstreamsStore";
import WorkstreamNameModal from "./WorkstreamNameModal.vue";

const store = useWorkstreamsStore();
const { workstreams, loaded, error } = storeToRefs(store);
const router = useRouter();

/** Controls the "New workstream" dialog; the page's navbar button opens it too. */
const createOpen = defineModel<boolean>("createOpen", { default: false });
const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

async function create(name: string): Promise<void> {
  const workstream = await store.create(name);
  await router.push({ name: "workstream", params: { id: workstream.id } });
}

const itemLabel = (count: number) => (count === 1 ? "1 item" : `${count} items`);

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

  <div v-if="workstreams.length" class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
    <ULink
      v-for="workstream in workstreams"
      :key="workstream.id"
      :to="`/workstreams/${workstream.id}`"
      class="block"
    >
      <UCard class="h-full transition-colors hover:bg-elevated/50">
        <div class="flex items-start gap-3">
          <UIcon name="i-lucide-layers" class="mt-0.5 size-5 shrink-0 text-primary" />
          <div class="min-w-0">
            <p class="truncate font-medium text-highlighted">{{ workstream.name }}</p>
            <p class="text-sm text-muted">
              {{ itemLabel(workstream.itemCount) }} · created
              {{ dateFormat.format(workstream.createdAt) }}
            </p>
          </div>
        </div>
      </UCard>
    </ULink>
  </div>

  <UEmpty
    v-else-if="loaded"
    class="py-16"
    icon="i-lucide-layers"
    title="No workstreams yet"
    description="Bundle related Jira issues (and soon pull requests and todos) into workstreams."
    :actions="[{ label: 'New workstream', icon: 'i-lucide-plus', onClick: () => (createOpen = true) }]"
  />

  <WorkstreamNameModal
    v-model:open="createOpen"
    title="New workstream"
    submit-label="Create"
    :save="create"
  />
</template>
