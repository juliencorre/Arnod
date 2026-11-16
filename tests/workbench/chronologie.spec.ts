import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { Timeline } from "@/types/ipc";

import { okOutcome } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Lot K (5.1-c) : chronologie d'un envoi, mesurée par le cœur. */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function sentWith(timeline: Timeline): Promise<AppWrapper> {
  const opened = await openProject((core) => {
    const outcome = okOutcome();
    outcome.response.timeline = timeline;
    outcome.response.totalMs = timeline.totalMs;
    core.workbench.nextSend = outcome;
  });
  await click(opened.app, '[role="treeitem"]', "Lister");
  await click(opened.app, "button", "Envoyer");
  await flushPromises();
  await click(opened.app, '[role="tab"]', "Chronologie");
  return opened.app;
}

function rows(wrapper: AppWrapper): [string, string][] {
  return wrapper
    .findAll(".timeline__row")
    .map((r) => [r.get(".timeline__label").text(), r.get(".timeline__ms").text()]);
}

describe("chronologie d'un envoi (5.1-c)", () => {
  it("nouvelle connexion HTTPS : DNS, connexion et TLS, attente, réception, total", async () => {
    app = await sentWith({
      dnsMs: 2,
      connectMs: 23,
      secure: true,
      serverMs: 16,
      receiveMs: 1,
      totalMs: 42,
    });
    expect({
      rows: rows(app),
      total: app.get(".timeline__total").text(),
      notes: app.findAll(".timeline__note").map((n) => n.text()),
      bars: app.findAll(".timeline__bar").length,
    }).toEqual({
      rows: [
        ["DNS", "2 ms"],
        ["Connexion et TLS", "23 ms"],
        ["Attente du serveur", "16 ms"],
        ["Réception", "1 ms"],
      ],
      total: "Total42 ms",
      notes: ["L'assertion de durée (responseTime) porte sur ce total."],
      bars: 4,
    });
  });

  it("connexion réutilisée : seulement l'attente et la réception, et c'est dit", async () => {
    app = await sentWith({
      dnsMs: null,
      connectMs: null,
      secure: false,
      serverMs: 30,
      receiveMs: 2,
      totalMs: 32,
    });
    expect({
      rows: rows(app).map(([label]) => label),
      reused: app.text().includes("Connexion réutilisée : ni DNS ni connexion pour cet envoi."),
    }).toEqual({ rows: ["Attente du serveur", "Réception"], reused: true });
  });
});
