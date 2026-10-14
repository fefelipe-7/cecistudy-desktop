/**
 * Testes de apresentação da tela Estudos.
 *
 * O rótulo do estado vem dos dados do Rust (`vencimento` null, flag
 * `relearning`) — se essas duas pontas divergirem, o erro aparece aqui, alto.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import type { CartaoDaFila } from "../data/bridge.ts";
import { AVALIACOES, RETENCAO_PROVISORIA, rotuloDaFila, textoVencimento } from "./view.ts";

function cartao(overrides: Partial<CartaoDaFila>): CartaoDaFila {
  return {
    id: "c1",
    conteudoId: "ct1",
    tituloConteudo: "Hippocampo",
    frente: "frente",
    verso: "verso",
    vencimento: null,
    relearning: false,
    ...overrides,
  };
}

test("rotuloDaFila: vencimento null é novo, a flag decide relearning", () => {
  assert.equal(rotuloDaFila(cartao({ vencimento: null })), "novo");
  assert.equal(rotuloDaFila(cartao({ vencimento: "2026-10-09", relearning: true })), "relearning");
  assert.equal(rotuloDaFila(cartao({ vencimento: "2026-10-09" })), "revisando");
});

test("textoVencimento: novo é primeiro encontro; agendado diz a data", () => {
  assert.equal(textoVencimento(null), "primeiro encontro — revise hoje");
  assert.equal(textoVencimento("2026-10-07"), "vencia em 2026-10-07");
});

test("as quatro notas existem, na ordem da crate", () => {
  assert.deepEqual(
    AVALIACOES.map((a) => a.id),
    [1, 2, 3, 4],
  );
  assert.equal(AVALIACOES[0]?.rotulo, "De novo");
  assert.equal(AVALIACOES[3]?.rotulo, "Fácil");
});

test("a retenção provisória é 0,9 e está entre as opções do seletor", () => {
  assert.equal(RETENCAO_PROVISORIA, 0.9);
  assert.ok(
    [0.75, 0.8, 0.85, 0.9, 0.95].includes(RETENCAO_PROVISORIA),
    "o padrão provisório precisa estar no seletor",
  );
});
