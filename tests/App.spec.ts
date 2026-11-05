import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { afterEach, describe, expect, it } from "vitest";

import App from "@/App.vue";
import { formatTime } from "@/lib/format";

function mountApp() {
  return mount(App, { global: { plugins: [createPinia()] } });
}

afterEach(() => {
  clearMocks();
});

describe("App", () => {
  it("affiche le texte renvoyé par la commande Rust au démarrage (F1, F2)", async () => {
    // Texte volontairement différent de « Hello World » : prouve qu'il n'est pas figé côté Vue.
    mockIPC((cmd) => {
      if (cmd === "hello") return { message: "Bonjour depuis Rust", timestampMs: 0 };
    });

    const wrapper = mountApp();
    expect(wrapper.get('[data-testid="message"]').text()).toBe("…");

    await flushPromises();

    expect(wrapper.get('[data-testid="message"]').text()).toBe("Bonjour depuis Rust");
  });

  it("rappelle la commande et affiche l'heure du cœur natif au clic sur « Dire bonjour » (F3)", async () => {
    const timestamps = [1_762_400_000_000, 1_762_400_005_250];
    let calls = 0;
    mockIPC((cmd) => {
      if (cmd !== "hello") return undefined;
      const timestampMs = timestamps[Math.min(calls, timestamps.length - 1)];
      calls += 1;
      return { message: "Hello World", timestampMs };
    });

    const wrapper = mountApp();
    await flushPromises();

    const time = () => wrapper.get('[data-testid="native-time"] time');
    expect(time().text()).toBe(formatTime(1_762_400_000_000));
    expect(time().attributes("datetime")).toBe(new Date(1_762_400_000_000).toISOString());

    const button = wrapper.get("button");
    expect(button.text()).toBe("Dire bonjour");
    await button.trigger("click");
    await flushPromises();

    expect(calls).toBe(2);
    expect(wrapper.get('[data-testid="message"]').text()).toBe("Hello World");
    expect(time().text()).toBe(formatTime(1_762_400_005_250));
  });

  it("affiche une alerte si le cœur natif ne répond pas", async () => {
    mockIPC(() => {
      throw "IPC indisponible";
    });

    const wrapper = mountApp();
    await flushPromises();

    expect(wrapper.get('[role="alert"]').text()).toBe(
      "La commande « hello » a échoué : IPC indisponible",
    );
    expect(wrapper.get("button").attributes("disabled")).toBeUndefined();
  });
});
