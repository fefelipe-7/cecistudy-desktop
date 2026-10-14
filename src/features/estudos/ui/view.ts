/**
 * Apresentação da tela Estudos (SPEC-D-010, D76, F8).
 *
 * Regra de apresentação, nada mais: rótulo e texto. O rótulo do estado vem dos
 * **dados** que o Rust devolve (`vencimento` null = novo; flag `relearning`), e
 * não de recálculo — o vencimento já chegou derivado (cláusula do vencimento).
 */

import type { CartaoDaFila } from "../data/bridge.ts";

/** As quatro notas, na ordem e nos valores da crate (1..4). */
export const AVALIACOES: ReadonlyArray<{ readonly id: 1 | 2 | 3 | 4; readonly rotulo: string }> = [
  { id: 1, rotulo: "De novo" },
  { id: 2, rotulo: "Difícil" },
  { id: 3, rotulo: "Bom" },
  { id: 4, rotulo: "Fácil" },
];

/**
 * A retenção desejada é parâmetro do motor; o **padrão** dela é **[P]** (§4.4).
 * A tela usa 0,9 provisório com seletor visível — decisão registrada como [P],
 * não fechada aqui.
 */
export const RETENCAO_PROVISORIA = 0.9;

export const RETENCOES: readonly number[] = [0.75, 0.8, 0.85, 0.9, 0.95];

/** O rótulo do estado do cartão na fila: novo, em relearning ou revisando. */
export function rotuloDaFila(cartao: CartaoDaFila): string {
  if (cartao.vencimento === null) return "novo";
  if (cartao.relearning) return "relearning";
  return "revisando";
}

/**
 * O texto de vencimento da fila. Todo cartão da fila **já é devido** (o Rust
 * filtrou); a data diz de quando é o vencimento que chegou.
 */
export function textoVencimento(vencimento: string | null): string {
  return vencimento === null ? "primeiro encontro — revise hoje" : `vencia em ${vencimento}`;
}
