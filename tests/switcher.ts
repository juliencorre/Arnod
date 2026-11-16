/**
 * Aides des tests pour le sélecteur de workspace (docs/plan-selecteur-workspace.md) :
 * le workspace affiché, la liste du menu, un choix.
 */
import { type DOMWrapper, flushPromises } from "@vue/test-utils";

import type { AppWrapper } from "./mountApp";

/** Nom affiché par le sélecteur : le workspace ouvert, ou « Tous les projets ». */
export function currentWorkspace(app: AppWrapper): string {
  return app.get(".switcher__label").text();
}

export function isSwitcherOpen(app: AppWrapper): boolean {
  return app.find('.switcher [role="menu"]').exists();
}

export async function openSwitcher(app: AppWrapper): Promise<void> {
  if (isSwitcherOpen(app)) return;
  await app.get(".switcher__trigger").trigger("click");
  await flushPromises();
}

export async function closeSwitcher(app: AppWrapper): Promise<void> {
  if (!isSwitcherOpen(app)) return;
  await app.get(".switcher__trigger").trigger("click");
  await flushPromises();
}

/** Entrées « Tous les projets » et workspaces du menu ouvert. */
export function switcherItems(app: AppWrapper): DOMWrapper<Element>[] {
  return app.findAll('.switcher__popover [role="menuitemradio"]');
}

export function switcherItem(app: AppWrapper, name: string): DOMWrapper<Element> {
  const found = switcherItems(app).find((item) => item.get(".switcher__name").text() === name);
  if (!found) throw new Error(`« ${name} » absent du sélecteur`);
  return found;
}

/** « Tous les projets » puis les workspaces, dans l'ordre du menu (qui se referme). */
export async function switcherNames(app: AppWrapper): Promise<string[]> {
  const wasOpen = isSwitcherOpen(app);
  await openSwitcher(app);
  const names = switcherItems(app).map((item) => item.get(".switcher__name").text());
  if (!wasOpen) await closeSwitcher(app);
  return names;
}

/** Ouvre un workspace, ou « Tous les projets », par le sélecteur. */
export async function chooseWorkspace(app: AppWrapper, name: string): Promise<void> {
  await openSwitcher(app);
  await switcherItem(app, name).trigger("click");
  await flushPromises();
}

/** Choisit une entrée de pied du menu : « Nouveau workspace », « Gérer les workspaces… ». */
export async function chooseSwitcherAction(app: AppWrapper, label: string): Promise<void> {
  await openSwitcher(app);
  const found = app
    .findAll('.switcher__popover [role="menuitem"]')
    .find((item) => item.get(".switcher__name").text() === label);
  if (!found) throw new Error(`« ${label} » absent du sélecteur`);
  await found.trigger("click");
  await flushPromises();
}
