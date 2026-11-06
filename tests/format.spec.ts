import { describe, expect, it } from "vitest";

import { countLabel, formatTime } from "@/lib/format";

describe("formatTime", () => {
  it("formate en heure locale française avec les millisecondes", () => {
    const date = new Date(2026, 10, 6, 9, 5, 7, 42);
    expect(formatTime(date.getTime())).toBe("09:05:07,042");
  });
});

describe("countLabel", () => {
  it("accorde au singulier jusqu'à 1, au pluriel au-delà", () => {
    expect(countLabel(0, "projet", "projets")).toBe("0 projet");
    expect(countLabel(1, "projet", "projets")).toBe("1 projet");
    expect(countLabel(6, "projet", "projets")).toBe("6 projets");
  });
});
