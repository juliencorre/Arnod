import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";
import { h } from "vue";

import SplitHandle from "@/components/zones/SplitHandle.vue";
import { dropRegion } from "@/lib/dnd";

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
});

/** Séparation dans un conteneur de 1 000 × 600 px. */
function mountHandle(props: Record<string, unknown> = {}) {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    width: 1000,
    height: 600,
    right: 1000,
    bottom: 600,
    toJSON: () => ({}),
  });
  return mount(
    {
      render: () =>
        h("div", [h(SplitHandle, { axis: "row", ratio: 0.5, label: "Largeur", ...props })]),
    },
    { attachTo: document.body },
  ).getComponent(SplitHandle);
}

function commits(handle: ReturnType<typeof mountHandle>): number[] {
  return (handle.emitted("commit") ?? []).map((args) => args[0] as number);
}

describe("SplitHandle (F4, F5)", () => {
  it("expose le motif « window splitter » accessible", () => {
    const handle = mountHandle({ ratio: 0.3 });
    expect(handle.attributes()).toMatchObject({
      role: "separator",
      tabindex: "0",
      "aria-label": "Largeur",
      "aria-orientation": "vertical",
      "aria-valuenow": "30",
      "aria-valuetext": "30 % · 70 %",
    });
  });

  it("se règle au clavier, avec les minimums en pixels (Z4)", async () => {
    const handle = mountHandle();
    await handle.trigger("keydown", { key: "ArrowRight" });
    await handle.trigger("keydown", { key: "ArrowLeft" });
    await handle.trigger("keydown", { key: "Home" });
    await handle.trigger("keydown", { key: "End" });
    await handle.trigger("keydown", { key: "Enter" });
    // 280 px minimum de chaque côté dans 1 000 px : entre 28 % et 72 %.
    expect(commits(handle)).toEqual([0.55, 0.45, 0.28, 0.72, 0.5]);
  });

  it("une séparation horizontale se règle avec les flèches haut et bas", async () => {
    const handle = mountHandle({ axis: "column", minFirst: 160, minSecond: 160 });
    expect(handle.attributes("aria-orientation")).toBe("horizontal");
    await handle.trigger("keydown", { key: "ArrowDown" });
    await handle.trigger("keydown", { key: "ArrowRight" });
    expect(commits(handle)).toEqual([0.55]);
  });

  it("glisser montre la taille des deux côtés, puis enregistre au relâchement", async () => {
    const handle = mountHandle({ names: ["test", "mock"] });
    await handle.trigger("pointerdown", { button: 0, pointerId: 1, clientX: 500 });
    await handle.trigger("pointermove", { pointerId: 1, clientX: 600 });
    expect(handle.emitted("preview")?.at(-1)).toEqual([0.6]);
    expect(handle.get(".handle__bubble").text()).toBe("test 600 px · mock 400 px");

    await handle.trigger("pointermove", { pointerId: 1, clientX: 950 });
    expect(handle.emitted("preview")?.at(-1)).toEqual([0.72]);

    await handle.trigger("pointerup", { pointerId: 1, clientX: 950 });
    expect(handle.emitted("preview")?.at(-1)).toEqual([null]);
    expect(commits(handle)).toEqual([0.72]);
    expect(handle.find(".handle__bubble").exists()).toBe(false);
  });

  it("double-clic : parts égales", async () => {
    const handle = mountHandle({ ratio: 0.7 });
    await handle.trigger("dblclick");
    expect(commits(handle)).toEqual([0.5]);
  });
});

describe("dropRegion (F2)", () => {
  const rect = { left: 0, top: 0, width: 1000, height: 600 } as DOMRect;

  it("bord droit, bord bas, sinon centre", () => {
    expect(dropRegion(rect, 900, 100)).toBe("right");
    expect(dropRegion(rect, 300, 550)).toBe("bottom");
    expect(dropRegion(rect, 500, 300)).toBe("center");
  });

  it("dans le coin, le bord le plus proche l'emporte", () => {
    expect(dropRegion(rect, 990, 450)).toBe("right");
    expect(dropRegion(rect, 750, 590)).toBe("bottom");
  });
});
