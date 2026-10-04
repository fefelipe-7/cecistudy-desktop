/**
 * Ações disponíveis para um item — a tabela única que decide o que o menu
 * oferece (`02-grade-desktop.md` Fase 2.5).
 *
 * ## Por que uma tabela e não uma condicional na tela
 *
 * A regra "quais ações existem" é função de `origin` × `state` × `kind`. Se ela
 * mora no componente, ela vira condicional espalhada, e o painel, a agenda e as
 * ações rápidas do mobile passam a discordar entre si. A tabela é pura: sem
 * React, sem `invoke`, sem relógio — então é testável com `node --test`, que é
 * o caminho que a spec 02 pede.
 *
 * ## O que esta tabela **não** é
 *
 * Ela **não** decide se a transição é legal. A legalidade é do Rust
 * (`ITEM_TRANSITIONS` + `guard_transition`), e o backend recusa mesmo que o menu
 * ofereça. Aqui a tabela usa o espelho `transitionAllowed` só para **não
 * oferecer** o que o Rust recusaria — oferecer um botão que volta erro é pior
 * que não tê-lo. As duas tabelas não podem divergir: `stateMachineParity.test.ts`
 * compara Rust e espelho par a par.
 *
 * É a distinção da SPEC-008 D1 do mobile: **habilitação ≠ imposição**. O menu
 * habilita; o Rust impõe.
 */

import { transitionAllowed } from "./state-machine.ts";
import type { ItemState, Origin } from "./types.ts";

/** Ações que o menu pode oferecer. */
export type ItemAction =
  /** Abrir o editor. Proibido em `google` (INV-6) e em estado encerrado. */
  | "editar"
  /** Apagar. Proibido em `google` (INV-6). */
  | "excluir"
  /** Mover na grade. */
  | "arrastar"
  /** Redimensionar na grade. */
  | "redimensionar"
  /** Registrar execução parcial: deixa o item em `em_andamento`. */
  | "registrar_execucao"
  /** Concluir: exige registro de execução (INV-3). */
  | "concluir"
  /** `concluido → em_andamento`. A única saída de `concluido`. */
  | "reabrir"
  /** Voltar a `planejado` a partir de um estado encerrado. */
  | "reativar";

/** O que a decisão depende. Tudo explícito: nada é lido de ambiente global. */
export interface MenuContext {
  readonly origin: Origin;
  readonly state: ItemState;
  /** Sobrescreve a leitura de origem somente-leitura. Só para teste. */
  readonly isReadOnlyOrigin?: boolean;
  /** `true` para ocorrência que continua ligada a uma regra de recorrência. */
  readonly hasRecurrenceRule?: boolean;
}

/** Estado encerrado: não se edita, não se arrasta (`02-grade-desktop.md:94`). */
const ENCERRADO: readonly ItemState[] = ["cancelado", "dispensado"];

/**
 * `google` é somente leitura (INV-6, D13). `guard_origin` recusa a escrita no
 * Rust, então o menu não pode oferecer o que o backend vai recusar.
 */
function somenteLeitura(context: MenuContext): boolean {
  return context.isReadOnlyOrigin ?? context.origin === "google";
}

/**
 * Regra de §4/D6: ocorrência com regra **não** muda em silêncio — qualquer
 * mudança passa primeiro pelo escopo ("esta ocorrência" ou "a série"). Por isso
 * `editar` e `arrastar` continuam disponíveis; o que muda é que o chamador tem
 * de pedir o escopo antes.
 */
export function requiresRecurrenceScope(context: MenuContext): boolean {
  return context.hasRecurrenceRule ?? false;
}

/**
 * A ação está disponível? Esta função é o **gate de habilitação**, não o de
 * imposição: o Rust recusa de novo qualquer transição fora da tabela.
 */
export function actionAllowed(action: ItemAction, context: MenuContext): boolean {
  const { state } = context;
  const encerrado = ENCERRADO.includes(state);

  if (somenteLeitura(context)) {
    // §10, 3ª linha: o item externo não é editável, mas pode gerar uma
    // responsabilidade própria — é assim que a Ceci se prepara para uma
    // reunião que não é dela. Registrar execução é o único caminho que resta,
    // e ele também escreve no item local, não no evento do Google.
    return action === "registrar_execucao";
  }

  switch (action) {
    case "editar":
    case "arrastar":
    case "redimensionar":
    case "excluir":
      return !encerrado;

    case "registrar_execucao":
      return transitionAllowed(state, "em_andamento");

    case "concluir":
      return transitionAllowed(state, "concluido");

    case "reabrir":
      // Reabrir é `concluido → em_andamento` e nada mais. Perguntar só
      // "posso ir para em_andamento" faria `reabrir` aparecer em qualquer estado
      // aberto, que não é o que a palavra significa.
      return state === "concluido" && transitionAllowed(state, "em_andamento");

    case "reativar":
      return transitionAllowed(state, "planejado");

    default: {
      // Ação nova sem regra aqui fica **indisponível**, não disponível por
      // acidente: o `never` é o que faz o compilador reclamar.
      const exaustivo: never = action;
      return exaustivo;
    }
  }
}

/** Ordem de apresentação do menu. */
const ORDEM: readonly ItemAction[] = [
  "editar",
  "arrastar",
  "redimensionar",
  "registrar_execucao",
  "concluir",
  "reabrir",
  "reativar",
  "excluir",
];

/** Ações que o menu oferece, na ordem de apresentação. */
export function actionsFor(context: MenuContext): readonly ItemAction[] {
  return ORDEM.filter((action) => actionAllowed(action, context));
}
