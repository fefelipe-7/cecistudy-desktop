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
 *
 * ⚠️ **Este arquivo é um espelho, não a fonte.** A regra é imposta em
 * `src-tauri/src/commands.rs` (`ITEM_TRANSITIONS` + `guard_transition`), no
 * caminho de escrita — é lá que uma transição ilegal é recusada. Aqui ela vive
 * para a **UI decidir o que oferecer** e para rodar teste sem o toolchain Rust.
 * Quando os dois divergirem, o Rust está certo e este arquivo é o bug.
 *
 * Antes de 2026-10-02 esta era a única implementação, e ela não validava
 * nada: não tinha chamador de produção, e o comentário do módulo Rust afirmava
 * que "quem valida a transição é o TypeScript, antes de chamar este arquivo" —
 * um "antes" que não existia. Transições ilegais eram graváveis de ponta a ponta.
 *
 * A lista abaixo e a do Rust têm de continuar idênticas.
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
