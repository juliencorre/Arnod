import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

import { AUTOSAVE_MS, requestKey, useWorkbenchStore } from "@/stores/workbench";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";

const PATHS = ["a.yml", "b.yml", "c.yml", "d.yml"];

let core: FakeCore;

/** Store avec quatre requêtes ouvertes dans « boutique-tests ». */
async function openedStore() {
  for (const path of PATHS) core.workbench.requests.set(path, newRequest(path));
  const workbench = useWorkbenchStore();
  for (const path of PATHS) await workbench.openEditor(BOUTIQUE_TESTS, path);
  return workbench;
}

function draftOf(workbench: ReturnType<typeof useWorkbenchStore>, path: string) {
  const draft = workbench.editors[requestKey(BOUTIQUE_TESTS, path)]?.draft;
  if (!draft) throw new Error(`brouillon absent : ${path}`);
  return draft;
}

function saves(): string[] {
  return core.calls.filter((c) => c.cmd === "request_save").map((c) => String(c.args.path));
}

beforeEach(() => {
  setActivePinia(createPinia());
  core = installFakeCore(boutiqueSnapshot());
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
});

describe("envoi d'une requête", () => {
  it("un second envoi demandé pendant l'enregistrement ne part pas", async () => {
    const workbench = await openedStore();
    const key = requestKey(BOUTIQUE_TESTS, "a.yml");
    draftOf(workbench, "a.yml").url = "{{baseUrl}}/modifiee";

    await Promise.all([workbench.send(key), workbench.send(key)]);

    expect(core.calls.filter((c) => c.cmd === "request_send")).toHaveLength(1);
    expect(workbench.editors[key]?.sending).toBe(false);
  });

  it("un enregistrement impossible est signalé sans empêcher l'envoi du brouillon", async () => {
    const workbench = await openedStore();
    const key = requestKey(BOUTIQUE_TESTS, "a.yml");
    draftOf(workbench, "a.yml").url = "{{baseUrl}}/modifiee";
    core.failNext("request_save", { kind: "io", message: "Disque plein." });

    await expect(workbench.send(key)).resolves.toBeUndefined();

    const sent = core.calls.find((c) => c.cmd === "request_send");
    expect((sent?.args.request as { url: string }).url).toBe("{{baseUrl}}/modifiee");
    expect(workbench.editors[key]?.saveState).toBe("error");
    expect(workbench.editors[key]?.saveError).toBe("Disque plein.");
    expect(workbench.editors[key]?.outcome?.response.status).toBe(200);
  });
});

describe("enregistrement automatique", () => {
  it("une modification ne sérialise que le brouillon modifié, une seule fois", async () => {
    const workbench = await openedStore();
    await nextTick();
    const stringify = vi.spyOn(JSON, "stringify");
    const drafts = (): number =>
      stringify.mock.calls.filter(([value]) => {
        return typeof value === "object" && value !== null && "assertions" in value;
      }).length;

    draftOf(workbench, "b.yml").url = "{{baseUrl}}/modifiee";
    await nextTick();
    // Lectures de l'état par l'interface : pastilles des onglets et de l'arborescence.
    for (const path of PATHS) workbench.isDirty(requestKey(BOUTIQUE_TESTS, path));
    for (const path of PATHS) workbench.isDirty(requestKey(BOUTIQUE_TESTS, path));

    expect(drafts()).toBe(1);
    expect(workbench.isDirty(requestKey(BOUTIQUE_TESTS, "b.yml"))).toBe(true);
    expect(workbench.isDirty(requestKey(BOUTIQUE_TESTS, "a.yml"))).toBe(false);
  });

  it("modifier une requête ne retarde pas l'enregistrement d'une autre", async () => {
    const workbench = await openedStore();
    await nextTick();
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    draftOf(workbench, "b.yml").url = "{{baseUrl}}/b";
    await nextTick();
    await vi.advanceTimersByTimeAsync(AUTOSAVE_MS - 100);
    draftOf(workbench, "a.yml").url = "{{baseUrl}}/a";
    await nextTick();
    await vi.advanceTimersByTimeAsync(150);

    // « b » est enregistrée 600 ms après sa propre modification, pas après celle de « a ».
    expect(saves()).toEqual(["b.yml"]);
    await vi.advanceTimersByTimeAsync(AUTOSAVE_MS);
    expect(saves()).toEqual(["b.yml", "a.yml"]);
    vi.useRealTimers();
    await flushPromises();
    expect(workbench.editors[requestKey(BOUTIQUE_TESTS, "a.yml")]?.saveState).toBe("saved");
  });

  it("une ligne de l'arborescence sans requête ouverte n'est jamais modifiée", async () => {
    const workbench = await openedStore();
    expect(workbench.isDirty(requestKey(BOUTIQUE_TESTS, "jamais-ouverte.yml"))).toBe(false);
  });
});
