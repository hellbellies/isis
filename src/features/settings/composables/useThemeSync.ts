import { watch, type Ref } from "vue";
import { useColorMode } from "@vueuse/core";
import type { Theme } from "@/types";

/**
 * Pushes the Rust-persisted theme into Nuxt UI's color mode (VueUse
 * `useColorMode`). Its localStorage cache applies the last theme on
 * startup before Rust responds, avoiding a flash.
 */
export function useThemeSync(theme: Ref<Theme | undefined>): void {
  const { store } = useColorMode();

  watch(theme, (value) => {
    if (value) store.value = value === "system" ? "auto" : value;
  });
}
