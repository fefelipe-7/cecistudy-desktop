import type { ItemState } from "./types.ts";

interface TransitionRow {
  readonly from: ItemState;
  readonly to: readonly ItemState[];
}

/**
 * §4 — a tabela completa e fechada de transições. A lista de `to` é o texto exato do
 * diagrama: nada mais é permitido, e nada aqui é final. `cancelado` e `dispensado`
 * voltam para `planejado` porque §4 os descreve como alteração externa e decisão
 * revisável, não como fim. Remover a **regra** de um item não é transição, é exclusão.
 */
const TRANSITIONS: readonly TransitionRow[] = [
  {
    from: "planejado",
    to: ["em_andamento", "concluido", "adiado", "nao_realizado", "cancelado", "dispensado"],
  },
  { from: "em_andamento", to: ["concluido", "adiado", "nao_realizado", "cancelado", "dispensado"] },
  {
    from: "adiado",
    to: ["planejado", "em_andamento", "concluido", "nao_realizado", "cancelado", "dispensado"],
  },
  {
    from: "nao_realizado",
    to: ["planejado", "em_andamento", "concluido", "adiado", "cancelado", "dispensado"],
  },
  { from: "concluido", to: ["em_andamento"] },
  { from: "cancelado", to: ["planejado"] },
  { from: "dispensado", to: ["planejado"] },
];

const BY_FROM: Readonly<Record<ItemState, readonly ItemState[]>> = TRANSITIONS.reduce(
  (accumulator, row) => {
    accumulator[row.from] = row.to;
    return accumulator;
  },
  {} as Record<ItemState, readonly ItemState[]>,
);

export function allowedTransitions(from: ItemState): readonly ItemState[] {
  return BY_FROM[from];
}

export function transitionAllowed(from: ItemState, to: ItemState): boolean {
  return BY_FROM[from].includes(to);
}
