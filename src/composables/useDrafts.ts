/**
 * Brouillons de la fenêtre (plan qualité, R6) : avant de fermer, détacher, rattacher, déplacer,
 * quitter ou installer, tout ce qui est modifié est enregistré ; ce qui ne peut pas l'être
 * (échec, conflit, secret en attente, source de contrat illisible) arrête l'action et le dit.
 */
import { useContractsStore } from "@/stores/contracts";
import { useToastStore } from "@/stores/toasts";
import { useWindowsStore } from "@/stores/windows";
import { useWorkbenchStore } from "@/stores/workbench";

/** Dernier segment d'un chemin, sans extension : le nom montré d'un fichier. */
function fileName(path: string): string {
  return (path.split("/").at(-1) ?? path).replace(/\.(ya?ml|json)$/, "");
}

/** Raison suivie d'un seul point final. */
function ending(reason: string): string {
  return `${reason.replace(/[.\s]+$/, "")}.`;
}

export function useDrafts() {
  const workbench = useWorkbenchStore();
  const contracts = useContractsStore();
  const toasts = useToastStore();

  /** Message d'un brouillon resté non enregistré, `null` s'il n'y en a pas. */
  function refusal(requests: string[], contractKeys: string[]): string | null {
    const first = requests[0];
    if (first !== undefined) {
      const editor = workbench.editors[first];
      const name = editor?.draft?.name ?? fileName(editor?.path ?? first);
      const others = requests.length + contractKeys.length - 1;
      const more = others > 0 ? ` (et ${String(others)} autre${others > 1 ? "s" : ""})` : "";
      return `« ${name} »${more} n'est pas enregistrée : ${ending(workbench.unsavedReason(first))}`;
    }
    const contract = contractKeys[0];
    if (contract !== undefined) {
      const entry = contracts.open[contract];
      const reason = ending(entry?.saveError ?? "illisible");
      return `Source du contrat « ${fileName(entry?.path ?? contract)} » non enregistrée : ${reason}`;
    }
    return null;
  }

  /** Enregistre une requête ; `false`, et la raison montrée, si elle ne l'est pas. */
  async function saveRequest(key: string): Promise<boolean> {
    if ((await workbench.save(key)) === "saved") return true;
    toasts.show(refusal([key], []) ?? "", { tone: "error" });
    return false;
  }

  /** Enregistre une source de contrat ; `false`, et la raison montrée, si elle ne l'est pas. */
  async function saveContract(key: string): Promise<boolean> {
    if (await contracts.save(key)) return true;
    toasts.show(refusal([], [key]) ?? "", { tone: "error" });
    return false;
  }

  /**
   * Enregistre contrats et requêtes modifiés de la fenêtre. `null` si tout l'est, sinon le
   * message à montrer (rien n'est montré ici : l'appelant choisit, notification ou erreur).
   */
  async function saveAll(): Promise<string | null> {
    const keys = Object.keys(contracts.open);
    const saved = await Promise.all(keys.map((key) => contracts.save(key)));
    const contractKeys = keys.filter((_, i) => !saved[i]);
    const requests = await workbench.saveAll();
    return refusal(requests, contractKeys);
  }

  /** Comme `saveAll`, la raison d'un refus montrée en notification ; `true` si tout est écrit. */
  async function saveAllOrSay(): Promise<boolean> {
    const message = await saveAll();
    if (message !== null) toasts.show(message, { tone: "error" });
    return message === null;
  }

  /**
   * « Rattacher » ou fermeture d'une fenêtre détachée (Z6, IN2, IN3) : enregistre, puis rend ses
   * onglets à la fenêtre principale ; un brouillon non enregistré garde la fenêtre ouverte.
   */
  async function reattach(): Promise<boolean> {
    if (!(await saveAllOrSay())) return false;
    await useWindowsStore().reattach();
    return true;
  }

  return { saveRequest, saveContract, saveAll, saveAllOrSay, reattach };
}
