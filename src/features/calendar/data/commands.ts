/**
 * `applyItemCommand` — o **único** ponto de entrada de escrita do Calendário
 * (`01-dominio-persistencia.md` Fase 1.6).
 *
 * ## Por que este arquivo mora em `data/` e não em `domain/`
 *
 * Ele nasceu em `domain/commands.ts` e isso estava errado. `AGENTS.md:17` diz que
 * `domain/` é puro — sem React, sem `invoke`, sem `Date` direto — e este arquivo
 * **é** `invoke`: ele não calcula nada, ele despacha para a ponte. Um arquivo que
 * não tem regra de domínio não é domínio, por mais bem escrito que esteja o
 * cabeçalho dele.
 *
 * `scripts/check-boundaries.mjs` acusou a violação (`domain/ não conhece a ponte
 * de dados`) e a acusação está certa. A correção não é afrouxar o gate: é pôr o
 * arquivo na camada que ele de fato ocupa.
 *
 * `data/` é a camada certa porque a regra dela (`AGENTS.md:16`) é exatamente esta:
 * falar o schema pelo IPC, sem reimplementar persistência nem regra de domínio.
 * Este arquivo não reimplementa nada — cada ramo vira uma chamada.
 *
 * ## Por que um despachante e não uma reimplementação
 *
 * Este arquivo **não** carrega a tabela de transições. A tabela é do Rust
 * (`ITEM_TRANSITIONS` + `guard_transition`), imposta no caminho de escrita, e esta
 * camada ficaria duplicando a regra se copiasse os 7 estados — que é exatamente o
 * débito C6 da recorrência: dois motores com capacidades diferentes, cada um
 * passando contra a sua própria cópia.
 *
 * Então o contrato aqui é deliberadamente estreito:
 *
 * 1. Cada ramo da união vira **uma** chamada de ponte. Nenhum ramo calcula estado,
 *    nem decide se a transição vale.
 * 2. Se o Rust recusar, a rejeição **sobe**. O chamador vê o erro do backend, não
 *    um `false` inventado aqui.
 * 3. `concluir` é o único ramo que carrega registro de execução (INV-3), e o Rust
 *    recusa qualquer outro caminho para `concluido`.
 *
 * É a distinção da SPEC-008 D1 do mobile aplicada ao backend: **habilitação ≠
 * imposição**. O menu (`domain/item-menu.ts`) habilita; este arquivo despacha; o
 * Rust impõe.
 */

import * as bridge from "./bridge.ts";
import type { Instant, ItemState, Minutes } from "../domain/types.ts";

/** Alvo de uma escrita. Espelha `ExecutionTargetWire` sem expor a ponte. */
export interface CommandTarget {
  readonly kind: "event" | "occurrence" | "responsibility" | "block";
  readonly id: string;
}

/**
 * Resultado do que aconteceu.
 *
 * Antes era `string`. Isso era mentira: o Rust aceita só estes dois valores
 * (`ExecutionResult` em `commands.rs`), e o tipo largo deixava a checagem para o
 * runtime — o chamador passava "consegui 80%", a ponte aceitava, e o backend
 * recusava. Tipo que aceita mais do que o dono da regra é o tipo que adia o erro
 * para onde ele custa mais caro.
 */
export type ExecutionResult = "concluido" | "parcial";

/** Registro do que aconteceu. Obrigatório só em `concluir` (INV-3). */
export interface ExecutionRecord {
  readonly startedAt: Instant;
  readonly finishedAt: Instant;
  readonly actualDuration: Minutes;
  readonly source: "manual" | "timer";
  readonly result?: ExecutionResult | undefined;
  readonly notes?: string | undefined;
}

/**
 * União discriminada de comandos.
 *
 * `startsAt` e `endsAt` são `Instant`, não `number`. Quando eram `number`, o
 * compilador recusava a passagem para a ponte — e o erro estava certo: o
 * glossário do grupo diz que `number` cru é o bug de ofuscação de tipo mais comum
 * do projeto, porque `number` é o único tipo que o sistema aceita em qualquer
 * lugar. Instante que é número volta a ser número na fronteira.
 */
export type ItemCommand =
  | { readonly kind: "set_state"; readonly target: CommandTarget; readonly to: ItemState }
  | {
      readonly kind: "record_execution";
      readonly target: CommandTarget;
      readonly record: ExecutionRecord;
    }
  | {
      readonly kind: "complete";
      readonly target: CommandTarget;
      readonly record: ExecutionRecord;
    }
  | {
      readonly kind: "move_occurrence";
      readonly id: string;
      readonly startsAt: Instant;
      readonly endsAt: Instant;
    }
  | {
      readonly kind: "reschedule_block";
      readonly id: string;
      readonly startsAt: Instant;
      readonly endsAt: Instant;
    }
  | {
      readonly kind: "delete_event";
      readonly id: string;
    }
  | {
      readonly kind: "update_event";
      readonly id: string;
      readonly patch: { readonly title?: string | undefined; readonly notes?: string | undefined };
    };

/**
 * A ponte, injetável.
 *
 * Não é abstração para teste: é o que permite provar que cada ramo chega ao
 * backend **sem** backend. `SPEC-D-001` `D49` exige que o gate de uma regra seja o
 * chamador de produção mais um golden — e um despachante que só se prova com o
 * Rust ligado não se prova na máquina em que o Rust não linka.
 */
export type PonteDeEscrita = Pick<
  typeof bridge,
  | "setItemState"
  | "recordExecution"
  | "completeItem"
  | "moveOccurrence"
  | "rescheduleBlock"
  | "deleteEvent"
  | "updateEvent"
>;

/**
 * Executa um comando de escrita.
 *
 * Rejeita a promessa se o backend recusar — a legality é do Rust, e a voz dele é a
 * que chega aqui. Não existe `try` neste arquivo por decisão: uma escrita que o
 * Rust recusou não tem "deu certo com aviso".
 *
 * `deps` tem padrão igual à ponte real, então o caminho de produção é exatamente
 * o mesmo que o teste exercita.
 */
export async function applyItemCommand(
  command: ItemCommand,
  deps: PonteDeEscrita = bridge,
): Promise<void> {
  switch (command.kind) {
    case "set_state":
      await deps.setItemState(command.target, command.to);
      return;

    case "record_execution":
      await deps.recordExecution(command.target, toExecutionInput(command.record));
      return;

    case "complete":
      await deps.completeItem(command.target, toExecutionInput(command.record));
      return;

    case "move_occurrence":
      await deps.moveOccurrence(command.id, command.startsAt, command.endsAt);
      return;

    case "reschedule_block":
      await deps.rescheduleBlock(command.id, command.startsAt, command.endsAt);
      return;

    case "delete_event":
      await deps.deleteEvent(command.id);
      return;

    case "update_event":
      await deps.updateEvent(command.id, {
        ...(command.patch.title !== undefined ? { title: command.patch.title } : {}),
        ...(command.patch.notes !== undefined ? { notes: command.patch.notes } : {}),
      });
      return;

    default: {
      // Comando novo sem ramo aqui é erro de compilação, não erro em runtime.
      const exaustivo: never = command;
      throw new Error(`comando não despachado: ${JSON.stringify(exaustivo)}`);
    }
  }
}

/**
 * Converte o registro do comando no input da ponte.
 *
 * O tipo de retorno é `ExecutionInput` **da ponte**, declarado uma vez só. Uma
 * versão anterior desta função declarava a forma à mão, e ela divergiu da ponte em
 * exatamente um campo — o mesmo modo de falha que ela existe para evitar.
 *
 * Os campos ausentes são **omitidos** em vez de enviados como `undefined`: sob
 * `exactOptionalPropertyTypes` (`tsconfig.json:25`) a diferença é de tipo, e no
 * payload é a diferença entre "não vim" e "vim vazio", que o Rust distingue.
 */
function toExecutionInput(record: ExecutionRecord): bridge.ExecutionInput {
  return {
    startedAt: record.startedAt,
    finishedAt: record.finishedAt,
    actualDuration: record.actualDuration,
    source: record.source,
    ...(record.result !== undefined ? { result: record.result } : {}),
    ...(record.notes !== undefined ? { notes: record.notes } : {}),
  };
}
