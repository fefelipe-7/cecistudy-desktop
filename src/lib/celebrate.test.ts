import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { CELEBRATION_KINDS, CONFETTI_COLORS, isCelebrationAllowed } from "./celebrate.ts";

describe("celebrate", () => {
  it("conhece 7 tipos de celebração, sem repetição", () => {
    assert.equal(CELEBRATION_KINDS.length, 7);
    assert.equal(new Set(CELEBRATION_KINDS).size, 7);
  });

  it("a paleta de confete é toda hex", () => {
    for (const cor of CONFETTI_COLORS) assert.match(cor, /^#[0-9A-Fa-f]{6}$/);
  });

  it("não celebra sob prefers-reduced-motion (D92 / §7)", () => {
    assert.equal(isCelebrationAllowed(true), false);
    assert.equal(isCelebrationAllowed(false), true);
  });
});
