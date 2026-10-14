import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { MASCOTE_EXPRESSION_NAMES } from "./mascote-expressions.ts";

/**
 * I2 de `SPEC-C-013` / `SPEC-D-014` §6: o desktop expõe exatamente as 24
 * expressões canônicas de `SPEC-C-013` §5.1, com os mesmos nomes do mobile
 * (`cecistudy/src/components/ui/Mascote.tsx`). Um nome renomeado aqui é a falha
 * de aceite, não um ajuste de implementação.
 */
const CANONICAS: readonly string[] = [
  "welcome-wave",
  "listening-hello",
  "focus-ready",
  "reading-curious",
  "library-shelf",
  "writing-note",
  "review-card",
  "quiz-ready",
  "correct-soft",
  "try-again",
  "empty-invite",
  "done-calm",
  "celebrate-small",
  "class-ready",
  "field-prepare",
  "supervision-reflect",
  "connection-link",
  "research-tcc",
  "writing-flow",
  "boundaries-care",
  "sync-wait",
  "loading-patient",
  "no-results",
  "pause-kind",
];

describe("MASCOTE_EXPRESSION_NAMES", () => {
  it("tem as 24 expressões canônicas de SPEC-C-013 §5.1, na ordem", () => {
    assert.deepEqual([...MASCOTE_EXPRESSION_NAMES], CANONICAS);
  });

  it("são 24 nomes únicos", () => {
    assert.equal(MASCOTE_EXPRESSION_NAMES.length, 24);
    assert.equal(new Set(MASCOTE_EXPRESSION_NAMES).size, MASCOTE_EXPRESSION_NAMES.length);
  });
});
