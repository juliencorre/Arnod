import { defineStore } from "pinia";
import { computed, ref } from "vue";

import { ipc } from "@/lib/ipc";
import { useNavigationStore } from "@/stores/navigation";
import type { SearchHit, SearchResults } from "@/types/ipc";

/** Délai après la dernière frappe avant de chercher (RH2). */
export const SEARCH_DELAY_MS = 150;

/**
 * Recherche Ctrl+K (écrans 1.5-a et 1.5-b, `docs/plan-v2.md`, lot A). Le cœur cherche et groupe
 * (`search_query`, R1 à R3) ; le store garde la saisie, les résultats et le résultat choisi au
 * clavier. La portée est le workspace affiché (« Tous les projets » : tous).
 */
export const useSearchStore = defineStore("search", () => {
  const navigation = useNavigationStore();
  const open = ref(false);
  const text = ref("");
  const results = ref<SearchResults | null>(null);
  /** Rang du résultat choisi parmi tous les résultats montrés. */
  const selected = ref(0);
  const error = ref<string | null>(null);
  /** La prochaine recherche relit les projets (première après l'ouverture). */
  let refresh = true;
  let timer: ReturnType<typeof setTimeout> | null = null;
  /** Numéro de la dernière recherche lancée : une réponse plus ancienne est ignorée. */
  let turn = 0;

  const scope = computed(() => navigation.currentWorkspaceId);
  /** Résultats montrés, dans l'ordre des groupes, pour ↑ ↓. */
  const hits = computed<SearchHit[]>(() => results.value?.groups.flatMap((g) => g.hits) ?? []);
  const current = computed(() => hits.value[selected.value] ?? null);

  function show(): void {
    open.value = true;
    refresh = true;
    if (text.value.trim() !== "") void run();
  }

  function close(): void {
    open.value = false;
    if (timer !== null) clearTimeout(timer);
    timer = null;
  }

  async function run(): Promise<void> {
    const mine = ++turn;
    try {
      const found = await ipc.searchQuery(scope.value, text.value, refresh);
      refresh = false;
      if (mine !== turn) return;
      results.value = found;
      selected.value = 0;
      error.value = null;
    } catch (caught) {
      if (mine === turn) error.value = caught instanceof Error ? caught.message : String(caught);
    }
  }

  /** Saisie : cherche 150 ms après la dernière frappe (RH2). */
  function type(next: string): void {
    text.value = next;
    if (timer !== null) clearTimeout(timer);
    if (next.trim() === "") {
      turn++;
      results.value = null;
      return;
    }
    timer = setTimeout(() => {
      timer = null;
      void run();
    }, SEARCH_DELAY_MS);
  }

  /** ↑ ↓ : résultat précédent ou suivant, en boucle. */
  function move(step: number): void {
    const count = hits.value.length;
    if (count === 0) return;
    selected.value = (selected.value + step + count) % count;
  }

  return { open, text, results, selected, error, scope, hits, current, show, close, type, move };
});
