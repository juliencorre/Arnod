import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";

import App from "@/App.vue";

/** Monte l'application complète (avec Pinia) et attend le chargement du catalogue. */
export async function mountApp() {
  const wrapper = mount(App, { global: { plugins: [createPinia()] }, attachTo: document.body });
  await flushPromises();
  return wrapper;
}

export type AppWrapper = Awaited<ReturnType<typeof mountApp>>;
