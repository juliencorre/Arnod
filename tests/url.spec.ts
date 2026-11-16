import { describe, expect, it } from "vitest";

import { pathParamNames } from "@/lib/url";

/** Mêmes cas que `r1_noms_des_parametres_de_chemin` côté Rust. */
describe("paramètres de chemin d'une adresse (R1)", () => {
  it("lit :nom et {nom}, dans l'ordre, sans doublon", () => {
    expect(pathParamNames("{{baseUrl}}/orders/:id/lines/{lineId}?x=:y#:z")).toEqual([
      "id",
      "lineId",
    ]);
    expect(pathParamNames("{{baseUrl}}/a/{{b}}/:id/:id")).toEqual(["id"]);
    expect(pathParamNames("https://h/a:b/:")).toEqual([]);
  });
});
