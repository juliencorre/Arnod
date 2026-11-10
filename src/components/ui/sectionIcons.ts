/**
 * Icônes des sections de la barre latérale (docs/plan-selecteur-workspace.md, I1, I2) :
 * tracés sur 16 px, en trait de 1,4 px, dessinés pour la maquette « Barre latérale ».
 * Les sections qui n'existent pas encore dans Arnod ont déjà la leur.
 */
export const SECTION_ICONS = {
  /** Requêtes d'un projet : une requête qui part. */
  requests: [
    "M9.5 3.2h-6a1 1 0 0 0-1 1v7.6a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1V8.6",
    "M8.2 7.8 13.6 2.4M10.2 2.4h3.4v3.4",
  ],
  /** Contrats de service : un document de code. */
  contracts: [
    "M3.5 1.8h5.7L12.5 5v9.2h-9V1.8Z",
    "M9.2 1.8V5h3.3",
    "M7 8.3 5.6 9.9 7 11.5M9 8.3l1.4 1.6L9 11.5",
  ],
  dashboard: ["M3 13V8.5M8 13V3M13 13V6"],
  products: ["M8 1.8 13.5 4.6v6.8L8 14.2 2.5 11.4V4.6L8 1.8Z", "M2.5 4.6 8 7.4l5.5-2.8M8 7.4v6.8"],
  requirements: [
    "M2.2 3.6l1.2 1.2 2-2.2M2.2 8.1l1.2 1.2 2-2.2M2.2 12.6l1.2 1.2 2-2.2",
    "M7.8 3.6h6M7.8 8.1h6M7.8 12.6h6",
  ],
  mocks: [
    "M2.8 3h10.4v4.4c0 3.1-2.2 5.6-5.2 6.4-3-.8-5.2-3.3-5.2-6.4V3Z",
    "M5.4 6.6h1.7M8.9 6.6h1.7M6.2 9.6c.5.6 1.1.9 1.8.9s1.3-.3 1.8-.9",
  ],
  templates: [
    "M4.1 2.5h7.8a1.6 1.6 0 0 1 1.6 1.6v7.8a1.6 1.6 0 0 1-1.6 1.6H4.1a1.6 1.6 0 0 1-1.6-1.6V4.1a1.6 1.6 0 0 1 1.6-1.6Z",
    "M2.5 6.3h11M6.7 6.3v7.2",
  ],
  tests: [
    "M6 2h4M6.8 2v4.2L3.4 12.2c-.5.8.1 1.8 1 1.8h7.2c.9 0 1.5-1 1-1.8L9.2 6.2V2",
    "M5.1 10h5.8",
  ],
  suites: [
    "M4.9 8a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM14.1 3.8a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0ZM14.1 12.2a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Z",
    "M4.9 8h1.6c1.6 0 1.5-4.2 3.1-4.2h1.5M6.5 8c1.6 0 1.5 4.2 3.1 4.2h1.5",
  ],
  campaigns: ["M4 14.2V2.4", "M4 3h8.2l-1.9 2.9L12.2 8.8H4"],
  anomalies: [
    "M8 5.3c-2 0-3.3 1.6-3.3 3.8S6 13.6 8 13.6s3.3-2.3 3.3-4.5S10 5.3 8 5.3Z",
    "M6.2 5.6c0-1.2.7-2.1 1.8-2.1s1.8.9 1.8 2.1M8 7.6v6M4.7 8.4H2.5M4.9 10.8l-2 1.1M11.3 8.4h2.2M11.1 10.8l2 1.1M5.4 6.4 3.9 5M10.6 6.4 12.1 5",
  ],
} as const satisfies Record<string, readonly string[]>;

export type SectionIconName = keyof typeof SECTION_ICONS;
