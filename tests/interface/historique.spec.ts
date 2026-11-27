/**
 * Version 1 (`docs/plan-v1.md`) : historique des envois (5.1-c bis). Chaque réponse reçue
 * rejoint l'historique de la requête, gardé sur le poste ; l'onglet « Historique » de la réponse
 * le liste, chaque envoi se déplie (corps, en-têtes, envoyé, assertions) et se compare au
 * précédent ; l'horloge de l'en-tête y mène ; « Effacer » se rattrape par « Annuler ».
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { formatSentAt } from "@/lib/format";

import type { FakeCore } from "../fakeCore";
import { okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function sendTwice(): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject();
  await click(opened.app, '[role="treeitem"]', "Santé");
  const base = okOutcome();
  opened.core.workbench.nextSend = okOutcome({
    response: {
      ...base.response,
      headers: [...base.response.headers, ["Location", "/sante/1"]],
      totalMs: 38,
    },
  });
  await opened.app.get("form.editor__line").trigger("submit");
  await flushPromises();
  opened.core.workbench.nextSend = okOutcome();
  await opened.app.get("form.editor__line").trigger("submit");
  await flushPromises();
  return opened;
}

describe("historique des envois (5.1-c bis)", () => {
  it("format des heures : aujourd'hui, hier, sinon le jour", () => {
    const now = new Date(2026, 10, 27, 10, 0).getTime();
    expect(formatSentAt(new Date(2026, 10, 27, 9, 14).getTime(), now)).toBe("auj. 09:14");
    expect(formatSentAt(new Date(2026, 10, 26, 17, 20).getTime(), now)).toBe("hier 17:20");
    expect(formatSentAt(new Date(2026, 10, 4, 11, 40).getTime(), now)).toBe("4 nov. 11:40");
  });

  it("chaque envoi rejoint l'onglet Historique, le plus récent déplié et comparé", async () => {
    let core: FakeCore;
    ({ app, core } = await sendTwice());
    expect(core.commands().filter((c) => c === "request_send")).toHaveLength(2);
    const tab = app
      .findAll('.response [role="tab"]')
      .find((t) => t.text().startsWith("Historique"));
    expect(tab?.text()).toBe("Historique 2");
    await tab?.trigger("click");
    await flushPromises();

    expect(app.get(".history__count").text()).toBe("2 envois sur ce poste, hors Git");
    const rows = app.findAll(".history__row");
    expect(rows.map((r) => r.attributes("aria-expanded"))).toEqual(["true", "false"]);
    expect(rows[0]?.text()).toContain("200");
    expect(rows[0]?.text()).toContain("45 ms");
    expect(rows[0]?.text()).toContain("1/1");
    const compare = app.get(".history__compare").text();
    expect(compare).toMatch(
      /^Comparé à auj\. \d\d:\d\d : l'en-tête Location a disparu ; 7 ms de plus\./,
    );

    await click(app, ".history__link", "Voir les différences");
    expect(core.commands()).toContain("history_diff");
    expect(app.findAll(".history__diff li").map((li) => li.text())).toEqual([
      "−Retiré : Location: /sante/1",
    ]);

    // Les parties de l'envoi déplié.
    await click(app, ".history__part", "En-têtes");
    expect(app.get(".history__pairs").text()).toContain("content-type");
    await click(app, ".history__part", "Assertions");
    expect(app.get(".history__assertions").text()).toContain("responseTime");
  });

  it("un envoi se déplie sur place, l'autre se replie", async () => {
    ({ app } = await sendTwice());
    await click(app, ".response__clock");
    const rows = app.findAll(".history__row");
    await rows[1]?.trigger("click");
    await flushPromises();
    expect(app.findAll(".history__row").map((r) => r.attributes("aria-expanded"))).toEqual([
      "false",
      "true",
    ]);
    // Le plus ancien n'a rien à quoi se comparer.
    expect(app.find(".history__compare").exists()).toBe(false);
  });

  it("l'horloge de l'en-tête ouvre l'historique", async () => {
    ({ app } = await sendTwice());
    expect(app.find(".history").exists()).toBe(false);
    const clock = app.get('.response__clock[aria-label="Historique des envois"]');
    await clock.trigger("click");
    await flushPromises();
    expect(app.find(".history").exists()).toBe(true);
    const selected = app
      .findAll('.response [role="tab"]')
      .find((t) => t.attributes("aria-selected") === "true");
    expect(selected?.text()).toBe("Historique 2");
  });

  it("« Effacer » vide l'historique, « Annuler » le remet", async () => {
    let core: FakeCore;
    ({ app, core } = await sendTwice());
    await click(app, ".response__clock");
    await click(app, ".history button", "Effacer");
    expect(app.get(".history__empty").text()).toContain("Aucun envoi gardé");
    expect(app.get(".toast__message").text()).toBe("2 envois effacés de l'historique.");
    await app.get(".toast__action").trigger("click");
    await flushPromises();
    expect(core.commands()).toContain("history_restore");
    await flushPromises();
    expect(app.findAll(".history__row")).toHaveLength(2);
  });
});
