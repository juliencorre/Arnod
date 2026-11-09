import { flushPromises } from "@vue/test-utils";

import { installFakeCore, type FakeCore } from "../fakeCore";
import { newRequest } from "../fakeWorkbench";
import { boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

/** Monte l'application sur « boutique-tests » avec un petit projet, et l'ouvre. */
export async function openProject(
  setup?: (core: FakeCore) => void,
): Promise<{ app: AppWrapper; core: FakeCore }> {
  const core = installFakeCore(boutiqueSnapshot());
  const wb = core.workbench;
  wb.folders.add("Clients");
  wb.requests.set("Clients/lister.yml", newRequest("Lister", { url: "{{baseUrl}}/clients" }));
  wb.requests.set("Clients/creer.yml", newRequest("Créer", { method: "POST" }));
  wb.requests.set("sante.yml", newRequest("Santé"));
  setup?.(core);
  const app = await mountApp();
  const card = app.findAll("main article").find((a) => a.find("h2").text() === "boutique-tests");
  const open = card?.findAll("button").find((b) => b.text().includes("Ouvrir"));
  if (!open) throw new Error("bouton d'ouverture introuvable");
  await open.trigger("click");
  await flushPromises();
  return { app, core };
}

export async function click(app: AppWrapper, selector: string, text?: string): Promise<void> {
  const elements = app.findAll(selector);
  const target = text === undefined ? elements[0] : elements.find((e) => e.text().includes(text));
  if (!target) throw new Error(`élément introuvable : ${selector} ${text ?? ""}`);
  await target.trigger("click");
  await flushPromises();
}
