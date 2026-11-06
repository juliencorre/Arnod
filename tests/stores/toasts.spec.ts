import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { IpcError } from "@/lib/ipc";
import { TOAST_DURATION_MS, useToastStore } from "@/stores/toasts";

beforeEach(() => {
  setActivePinia(createPinia());
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useToastStore", () => {
  it("affiche une notification puis la retire après le délai", () => {
    const toasts = useToastStore();

    toasts.show("boutique-tests créé dans le workspace Mes projets");

    expect(toasts.current).toMatchObject({
      message: "boutique-tests créé dans le workspace Mes projets",
      tone: "info",
      action: null,
    });
    vi.advanceTimersByTime(TOAST_DURATION_MS - 1);
    expect(toasts.current).not.toBeNull();
    vi.advanceTimersByTime(1);
    expect(toasts.current).toBeNull();
  });

  it("remplace la notification précédente et repart pour un délai complet", () => {
    const toasts = useToastStore();
    toasts.show("première");
    vi.advanceTimersByTime(TOAST_DURATION_MS - 100);

    toasts.show("seconde");
    vi.advanceTimersByTime(200);

    expect(toasts.current?.message).toBe("seconde");
  });

  it("suspend la disparition pendant le survol", () => {
    const toasts = useToastStore();
    toasts.show("Workspace supprimé");

    toasts.pause();
    vi.advanceTimersByTime(TOAST_DURATION_MS * 3);
    expect(toasts.current).not.toBeNull();

    toasts.resume();
    vi.advanceTimersByTime(TOAST_DURATION_MS);
    expect(toasts.current).toBeNull();
  });

  it("exécute l'action puis ferme la notification", async () => {
    const toasts = useToastStore();
    const run = vi.fn();
    toasts.show("Projet retiré", { action: { label: "Annuler", run } });

    await toasts.runAction();

    expect(run).toHaveBeenCalledOnce();
    expect(toasts.current).toBeNull();
  });

  it("notifie l'échec de l'action", async () => {
    const toasts = useToastStore();
    const failure = new IpcError("undo", {
      kind: "undoExpired",
      message: "Cette action ne peut plus être annulée.",
    });
    toasts.show("Projet retiré", {
      action: {
        label: "Annuler",
        run: () => Promise.reject(failure),
      },
    });

    await toasts.runAction();

    expect(toasts.current).toMatchObject({
      message: "Cette action ne peut plus être annulée.",
      tone: "error",
    });
  });

  it("affiche une erreur quelconque comme texte", () => {
    const toasts = useToastStore();

    toasts.showError("disque plein");

    expect(toasts.current).toMatchObject({ message: "disque plein", tone: "error" });
  });
});
