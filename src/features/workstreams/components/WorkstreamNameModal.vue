<script setup lang="ts">
import { ref, watch } from "vue";
import type { AppError } from "@/types";

const props = defineProps<{
  title: string;
  submitLabel: string;
  initialName?: string;
  /** Persists the name; rejecting with an `AppError` keeps the modal open and shows the message. */
  save: (name: string) => Promise<unknown>;
}>();

const open = defineModel<boolean>("open", { default: false });

const name = ref("");
const error = ref<string | null>(null);
const saving = ref(false);

watch(open, (isOpen) => {
  if (!isOpen) return;
  name.value = props.initialName ?? "";
  error.value = null;
});

async function submit(): Promise<void> {
  saving.value = true;
  error.value = null;
  try {
    await props.save(name.value);
    open.value = false;
  } catch (err: unknown) {
    error.value = (err as AppError).message;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <UModal v-model:open="open" :title="title">
    <template #body>
      <form id="workstream-name-form" @submit.prevent="submit">
        <UFormField label="Name" :error="error ?? undefined">
          <UInput v-model="name" autofocus placeholder="e.g. Checkout redesign" class="w-full" />
        </UFormField>
      </form>
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="ghost" label="Cancel" @click="open = false" />
        <UButton type="submit" form="workstream-name-form" :label="submitLabel" :loading="saving" />
      </div>
    </template>
  </UModal>
</template>
