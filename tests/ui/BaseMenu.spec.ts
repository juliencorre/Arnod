import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { h } from "vue";

import BaseMenu from "@/components/ui/BaseMenu.vue";
import type { MenuItem, MenuTriggerAttrs } from "@/components/ui/menu";

/** Menu ⋯ d'un projet (maquette 2a). */
const items: MenuItem[] = [
  { id: "open", label: "Ouvrir" },
  {
    id: "add",
    label: "Ajouter à un autre workspace",
    children: [
      { id: "add:mes-projets", label: "Mes projets", checked: true, disabled: true },
      { id: "add:facturation", label: "Facturation & partenaires", checked: false },
    ],
  },
  { id: "reveal", label: "Afficher dans le référentiel" },
  { id: "unavailable", label: "Indisponible", disabled: true },
  {
    id: "remove",
    label: "Retirer de « Boutique »",
    hint: "Le projet reste dans le référentiel.",
    tone: "danger",
    separatorBefore: true,
  },
];

function mountMenu() {
  return mount(BaseMenu, {
    props: { items, label: "Actions du projet boutique-tests" },
    slots: {
      trigger: ({ attrs }: { attrs: MenuTriggerAttrs }) =>
        h("button", { ...attrs, type: "button", class: "trigger" }, "⋯"),
    },
    attachTo: document.body,
  });
}

type MenuWrapper = ReturnType<typeof mountMenu>;

function focusedText(): string {
  const focused = document.activeElement;
  return focused?.querySelector(".menu__label")?.textContent ?? focused?.textContent ?? "";
}

/** Appuie sur une touche dans l'élément qui a le focus. */
async function press(key: string): Promise<void> {
  document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  await flushPromises();
}

async function openWithClick(wrapper: MenuWrapper): Promise<void> {
  const trigger = wrapper.get<HTMLButtonElement>(".trigger");
  trigger.element.focus();
  await trigger.trigger("click");
  await flushPromises();
}

let wrapper: MenuWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
});

describe("BaseMenu", () => {
  it("s'ouvre au clic et met le focus sur la première entrée", async () => {
    wrapper = mountMenu();
    const trigger = wrapper.get(".trigger");
    expect(trigger.attributes("aria-haspopup")).toBe("menu");
    expect(trigger.attributes("aria-expanded")).toBe("false");

    await openWithClick(wrapper);

    expect(trigger.attributes("aria-expanded")).toBe("true");
    const menu = wrapper.get('[role="menu"]');
    expect(menu.attributes("aria-label")).toBe("Actions du projet boutique-tests");
    expect(trigger.attributes("aria-controls")).toBe(menu.attributes("id"));
    expect(focusedText()).toBe("Ouvrir");
  });

  it("se parcourt aux flèches en sautant les entrées désactivées", async () => {
    wrapper = mountMenu();
    await openWithClick(wrapper);

    const visited: string[] = [];
    for (let i = 0; i < 4; i++) {
      await press("ArrowDown");
      visited.push(focusedText());
    }
    expect(visited).toEqual([
      "Ajouter à un autre workspace",
      "Afficher dans le référentiel",
      "Retirer de « Boutique »",
      "Ouvrir",
    ]);

    await press("ArrowUp");
    expect(focusedText()).toBe("Retirer de « Boutique »");
    await press("Home");
    expect(focusedText()).toBe("Ouvrir");
    await press("End");
    expect(focusedText()).toBe("Retirer de « Boutique »");
  });

  it("Entrée choisit l'entrée, ferme le menu et rend le focus au déclencheur", async () => {
    wrapper = mountMenu();
    await openWithClick(wrapper);
    await press("ArrowDown");
    await press("ArrowDown");

    await press("Enter");

    expect(wrapper.emitted("select")).toEqual([["reveal"]]);
    expect(wrapper.find('[role="menu"]').exists()).toBe(false);
    expect(document.activeElement).toBe(wrapper.get(".trigger").element);
  });

  it("ouvre le sous-menu à droite et le referme à gauche", async () => {
    wrapper = mountMenu();
    await openWithClick(wrapper);
    await press("ArrowDown");

    await press("ArrowRight");

    const parent = wrapper.get('[data-level="0"][data-index="1"]');
    expect(parent.attributes("aria-expanded")).toBe("true");
    const children = wrapper.findAll('[data-level="1"]');
    expect(children.map((child) => child.attributes("role"))).toEqual([
      "menuitemcheckbox",
      "menuitemcheckbox",
    ]);
    expect(children.map((child) => child.attributes("aria-checked"))).toEqual(["true", "false"]);
    // « Mes projets » contient déjà le projet : désactivé, le focus va au suivant.
    expect(focusedText()).toBe("Facturation & partenaires");

    await press("ArrowLeft");
    expect(wrapper.find('[data-level="1"]').exists()).toBe(false);
    expect(focusedText()).toBe("Ajouter à un autre workspace");

    await press("Enter");
    await press("Enter");
    expect(wrapper.emitted("select")).toEqual([["add:facturation"]]);
  });

  it("Échap ferme sans propager l'événement (une fenêtre parente reste ouverte)", async () => {
    wrapper = mountMenu();
    let propagated = false;
    const listener = (): void => {
      propagated = true;
    };
    document.body.addEventListener("keydown", listener);
    await openWithClick(wrapper);

    await press("Escape");

    document.body.removeEventListener("keydown", listener);
    expect(propagated).toBe(false);
    expect(wrapper.find('[role="menu"]').exists()).toBe(false);
    expect(document.activeElement).toBe(wrapper.get(".trigger").element);
  });

  it("flèche haut sur le déclencheur ouvre sur la dernière entrée", async () => {
    wrapper = mountMenu();
    wrapper.get<HTMLButtonElement>(".trigger").element.focus();

    await press("ArrowUp");

    expect(focusedText()).toBe("Retirer de « Boutique »");
  });

  it("se ferme sur un clic à l'extérieur", async () => {
    wrapper = mountMenu();
    await openWithClick(wrapper);

    document.body.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    await flushPromises();

    expect(wrapper.find('[role="menu"]').exists()).toBe(false);
  });

  it("choisit à la souris et ignore les entrées désactivées", async () => {
    wrapper = mountMenu();
    await openWithClick(wrapper);

    await wrapper.get('[data-index="3"]').trigger("click");
    expect(wrapper.emitted("select")).toBeUndefined();
    expect(wrapper.get('[data-index="3"]').attributes("aria-disabled")).toBe("true");

    const remove = wrapper.get('[data-index="4"]');
    expect(remove.classes()).toContain("menu__item--danger");
    expect(remove.text()).toContain("Le projet reste dans le référentiel.");
    await remove.trigger("click");
    expect(wrapper.emitted("select")).toEqual([["remove"]]);
  });
});
