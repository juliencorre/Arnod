import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import BaseButton from "@/components/ui/BaseButton.vue";
import BaseDialog from "@/components/ui/BaseDialog.vue";
import ColorDot from "@/components/ui/ColorDot.vue";
import SearchInput from "@/components/ui/SearchInput.vue";
import ToastHost from "@/components/ui/ToastHost.vue";
import ToggleSwitch from "@/components/ui/ToggleSwitch.vue";
import { TOAST_DURATION_MS, useToastStore } from "@/stores/toasts";

describe("BaseButton", () => {
  it("rend un bouton de type button avec sa variante", () => {
    const wrapper = mount(BaseButton, {
      props: { variant: "primary" },
      slots: { default: "Créer le projet" },
    });

    const button = wrapper.get("button");
    expect(button.attributes("type")).toBe("button");
    expect(button.classes()).toContain("button--primary");
    expect(button.text()).toBe("Créer le projet");
  });
});

describe("BaseDialog", () => {
  function mountDialog(open: boolean) {
    return mount(BaseDialog, {
      props: {
        open,
        title: "Nouveau projet",
        description: "Il sera rangé dans le workspace ouvert et dans le workspace par défaut.",
      },
      slots: {
        default: "<label>Nom du projet <input /></label>",
        footer: "<button>Annuler</button>",
      },
      attachTo: document.body,
    });
  }

  it("s'ouvre en modale avec un titre et une description accessibles", () => {
    const wrapper = mountDialog(true);
    const dialog = wrapper.get<HTMLDialogElement>("dialog");

    expect(dialog.element.open).toBe(true);
    const title = wrapper.get("h2");
    expect(title.text()).toBe("Nouveau projet");
    expect(dialog.attributes("aria-labelledby")).toBe(title.attributes("id"));
    expect(dialog.attributes("aria-describedby")).toBe(wrapper.get("p").attributes("id"));
    expect(wrapper.find("footer").text()).toBe("Annuler");
    wrapper.unmount();
  });

  it("Échap demande la fermeture sans fermer d'elle-même", async () => {
    const wrapper = mountDialog(true);
    const dialog = wrapper.get<HTMLDialogElement>("dialog");

    const cancel = new Event("cancel", { cancelable: true });
    dialog.element.dispatchEvent(cancel);

    expect(cancel.defaultPrevented).toBe(true);
    expect(wrapper.emitted("close")).toHaveLength(1);
    expect(dialog.element.open).toBe(true);

    await wrapper.setProps({ open: false });
    expect(dialog.element.open).toBe(false);
    wrapper.unmount();
  });

  it("un clic sur le fond demande la fermeture, pas un clic dans le panneau", async () => {
    const wrapper = mountDialog(true);

    await wrapper.get("input").trigger("click");
    expect(wrapper.emitted("close")).toBeUndefined();

    await wrapper.get("dialog").trigger("click");
    expect(wrapper.emitted("close")).toHaveLength(1);
    wrapper.unmount();
  });

  it("s'ouvre quand `open` passe à vrai", async () => {
    const wrapper = mountDialog(false);
    const dialog = wrapper.get<HTMLDialogElement>("dialog");
    expect(dialog.element.open).toBe(false);

    await wrapper.setProps({ open: true });

    expect(dialog.element.open).toBe(true);
    wrapper.unmount();
  });
});

describe("ToggleSwitch", () => {
  it("expose l'état par role=switch et bascule au clic", async () => {
    const wrapper = mount(ToggleSwitch, {
      props: { modelValue: false },
      slots: { default: "Workspace par défaut" },
    });
    const toggle = wrapper.get('[role="switch"]');
    expect(toggle.attributes("aria-checked")).toBe("false");
    expect(wrapper.text()).toContain("Workspace par défaut");

    await toggle.trigger("click");

    expect(wrapper.emitted("update:modelValue")).toEqual([[true]]);
  });

  it("ne bascule pas désactivé", async () => {
    const wrapper = mount(ToggleSwitch, { props: { modelValue: true, disabled: true } });

    await wrapper.get('[role="switch"]').trigger("click");

    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
  });
});

describe("ColorDot", () => {
  it("prend la couleur du jeton du workspace et reste décorative", () => {
    const wrapper = mount(ColorDot, { props: { color: "teal" } });
    const dot = wrapper.get("span");

    expect(dot.attributes("style")).toContain("var(--color-ws-teal)");
    expect(dot.attributes("aria-hidden")).toBe("true");
  });
});

describe("SearchInput", () => {
  function mountSearch(modelValue: string) {
    return mount(SearchInput, {
      props: {
        modelValue,
        label: "Rechercher un workspace",
        "onUpdate:modelValue": (value: string) => wrapper.setProps({ modelValue: value }),
      },
    });
  }
  let wrapper: ReturnType<typeof mountSearch>;

  it("met à jour la recherche et porte un libellé accessible", async () => {
    wrapper = mountSearch("");
    const input = wrapper.get("input");
    expect(input.attributes("aria-label")).toBe("Rechercher un workspace");
    expect(input.attributes("placeholder")).toBe("Rechercher un workspace");

    await input.setValue("bout");

    expect(wrapper.props("modelValue")).toBe("bout");
  });

  it("Échap vide le champ sans propager, puis laisse passer s'il est vide", async () => {
    wrapper = mountSearch("bout");
    const input = wrapper.get("input");

    const first = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    input.element.dispatchEvent(first);
    await flushPromises();
    expect(first.defaultPrevented).toBe(true);
    expect(wrapper.props("modelValue")).toBe("");

    const second = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    input.element.dispatchEvent(second);
    expect(second.defaultPrevented).toBe(false);
  });
});

describe("ToastHost", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("annonce la notification et exécute son action", async () => {
    const wrapper = mount(ToastHost);
    const toasts = useToastStore();
    const run = vi.fn();

    toasts.show("Workspace « Recette sprint 42 » supprimé", {
      action: { label: "Annuler", run },
    });
    await flushPromises();

    expect(wrapper.get('[role="status"]').attributes("aria-live")).toBe("polite");
    expect(wrapper.text()).toContain("Workspace « Recette sprint 42 » supprimé");
    await wrapper.get("button").trigger("click");
    await flushPromises();

    expect(run).toHaveBeenCalledOnce();
    expect(wrapper.find(".toast").exists()).toBe(false);
  });

  it("reste affichée tant qu'elle est survolée", async () => {
    const wrapper = mount(ToastHost);
    const toasts = useToastStore();
    toasts.show("Projet retiré");
    await flushPromises();

    await wrapper.get(".toast").trigger("mouseenter");
    vi.advanceTimersByTime(TOAST_DURATION_MS * 2);
    await flushPromises();
    expect(wrapper.find(".toast").exists()).toBe(true);

    await wrapper.get(".toast").trigger("mouseleave");
    vi.advanceTimersByTime(TOAST_DURATION_MS);
    await flushPromises();
    expect(wrapper.find(".toast").exists()).toBe(false);
  });

  it("distingue les erreurs", async () => {
    const wrapper = mount(ToastHost);
    useToastStore().showError(new Error("disque plein"));
    await flushPromises();

    expect(wrapper.get(".toast").classes()).toContain("toast--error");
    expect(wrapper.find("button").exists()).toBe(false);
  });
});
