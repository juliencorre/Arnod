import { defineStore } from "pinia";
import { ref } from "vue";

import { ipc } from "@/lib/ipc";
import type { HelloResponse } from "@/types/ipc";

/** État de la démonstration « hello » : dernière réponse du cœur natif. */
export const useHelloStore = defineStore("hello", () => {
  const response = ref<HelloResponse | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  /** Appelle la commande Rust `hello` et mémorise sa réponse. */
  async function sayHello(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      response.value = await ipc.hello();
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading.value = false;
    }
  }

  return { response, loading, error, sayHello };
});
