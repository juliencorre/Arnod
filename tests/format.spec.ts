import { describe, expect, it } from "vitest";

import { formatTime } from "@/lib/format";

describe("formatTime", () => {
  it("formate en heure locale française avec les millisecondes", () => {
    const date = new Date(2026, 10, 6, 9, 5, 7, 42);
    expect(formatTime(date.getTime())).toBe("09:05:07,042");
  });
});
