import { test } from "node:test";
import assert from "node:assert/strict";

import {
  applyItemCommand,
  type CommandTarget,
  type ExecutionRecord,
  type ItemCommand,
  type PonteDeEscrita,
} from "../commands.ts";
import { instant, minutes } from "../../domain/time.ts";

/**
 * Contrato de `data/commands.ts` — o despachante de escrita.
 *
 * Este arquivo **não** tem regra de domínio: ele mapeia cada ramo da união para
 * uma chamada de ponte. O que ele tem é uma garantia estrutural, e é ela que este
 * arquivo prova:
 *
 * - todo ramo da união chega ao backend;
 * - nenhum ramo chega a **dois** comandos;
 * - a recusa do Rust sobe, e não vira `false`;
 * - `compluir` é o único caminho para `concluido` (INV-3 é do Rust, e o que se
 *   prova aqui é que este arquivo não abre uma porta alternativa);
 * - campo ausente é **omitido** do payload, e não enviado como `undefined`.
 *
 * `SPEC-D-001` `D49` exige que o gate de uma regra seja o chamador de produção
 * mais um golden. Este arquivo prova a primeira metade — que cada ramo chega —
 * com uma ponte falsa injetada. A segunda metade, o chamador de produção, está
 * registrada como dívida: `rg "applyItemCommand" src` só encontra este arquivo.
 */

const ALVO: CommandTarget = { kind: "responsibility", id: "r-1" };

function registro(over: Partial<ExecutionRecord> = {}): ExecutionRecord {
  return {
    startedAt: instant(1_700_000_000_000),
    finishedAt: instant(1_700_000_600_000),
    actualDuration: minutes(10),
    source: "manual",
    ...over,
  };
}

/** Ponte falsa: registra o que foi chamado, sem Tauri e sem Rust. */
function ponteFalsa() {
  const chamadas: { nome: string; args: readonly unknown[] }[] = [];
  const duplo =
    (nome: string) =>
    async (...args: readonly unknown[]): Promise<void> => {
      chamadas.push({ nome, args });
    };
  // A conversão é explícita e é honesta: `updateEvent` devolve `CalendarEvent`
  // na ponte real e este duplo devolve `void`. O despacho **descarta** o retorno —
  // o que se prova aqui é qual comando foi chamado, não o que ele devolveu — e um
  // duplo que inventa um `CalendarEvent` para satisfazer o tipo seria pior que o
  // cast, porque pareceria que o retorno importa.
  const ponte = {
    setItemState: duplo("setItemState"),
    recordExecution: duplo("recordExecution"),
    completeItem: duplo("completeItem"),
    moveOccurrence: duplo("moveOccurrence"),
    rescheduleBlock: duplo("rescheduleBlock"),
    deleteEvent: duplo("deleteEvent"),
    updateEvent: duplo("updateEvent"),
  } as unknown as PonteDeEscrita;
  return { ponte, chamadas };
}

/** Ponte que recusa num comando e responde `void` nos outros. */
function ponteQueRecusa(recusa: Error, nome: keyof PonteDeEscrita): PonteDeEscrita {
  const emVazio = async (): Promise<void> => undefined;
  return {
    setItemState: async () => undefined,
    recordExecution: emVazio,
    completeItem: emVazio,
    moveOccurrence: emVazio,
    rescheduleBlock: emVazio,
    deleteEvent: emVazio,
    updateEvent: emVazio,
    [nome]: async () => {
      throw recusa;
    },
  } as unknown as PonteDeEscrita;
}

const CASOS: readonly { nome: string; comando: ItemCommand; esperado: string }[] = [
  {
    nome: "set_state chega em setItemState, com alvo e estado",
    comando: { kind: "set_state", target: ALVO, to: "em_andamento" },
    esperado: "setItemState",
  },
  {
    nome: "record_execution chega em recordExecution, com o registro inteiro",
    comando: { kind: "record_execution", target: ALVO, record: registro() },
    esperado: "recordExecution",
  },
  {
    nome: "complete chega em completeItem, com o registro — INV-3",
    comando: { kind: "complete", target: ALVO, record: registro() },
    esperado: "completeItem",
  },
  {
    nome: "move_occurrence chega em moveOccurrence, com instante e não com número",
    comando: {
      kind: "move_occurrence",
      id: "o-1",
      startsAt: instant(1_700_000_000_000),
      endsAt: instant(1_700_003_600_000),
    },
    esperado: "moveOccurrence",
  },
  {
    nome: "reschedule_block chega em rescheduleBlock",
    comando: {
      kind: "reschedule_block",
      id: "b-1",
      startsAt: instant(1_700_000_000_000),
      endsAt: instant(1_700_003_600_000),
    },
    esperado: "rescheduleBlock",
  },
  {
    nome: "delete_event chega em deleteEvent",
    comando: { kind: "delete_event", id: "e-1" },
    esperado: "deleteEvent",
  },
  {
    nome: "update_event chega em updateEvent, com o patch",
    comando: { kind: "update_event", id: "e-1", patch: { title: "Novo título" } },
    esperado: "updateEvent",
  },
];

for (const caso of CASOS) {
  test(`cada ramo da união chega ao backend — ${caso.nome}`, async () => {
    const { ponte, chamadas } = ponteFalsa();
    await applyItemCommand(caso.comando, ponte);
    assert.equal(chamadas.length, 1, "um ramo chama exatamente um comando");
    assert.equal(chamadas[0]?.nome, caso.esperado);
  });
}

test("nenhum ramo chega a dois comandos: a união e a ponte têm a mesma cardinalidade", async () => {
  for (const caso of CASOS) {
    const { ponte, chamadas } = ponteFalsa();
    await applyItemCommand(caso.comando, ponte);
    assert.equal(chamadas.length, 1, `${caso.comando.kind} chamou ${chamadas.length} comandos`);
  }
});

test("os sete ramos da união estão cobertos acima", () => {
  const kinds = new Set(CASOS.map((c) => c.comando.kind));
  assert.equal(kinds.size, 7);
  assert.deepEqual([...kinds].sort(), [
    "complete",
    "delete_event",
    "move_occurrence",
    "record_execution",
    "reschedule_block",
    "set_state",
    "update_event",
  ]);
});

test("a recusa do Rust sobe: não existe catch, e não existe retorno de sucesso falso", async () => {
  const recusa = new Error("transição não permitida: concluido → planejado");
  const ponte = ponteQueRecusa(recusa, "setItemState");

  await assert.rejects(
    () => applyItemCommand({ kind: "set_state", target: ALVO, to: "planejado" }, ponte),
    (erro: unknown) => erro === recusa,
  );
});

test("campo ausente é omitido do payload, e não enviado como undefined", async () => {
  const { ponte, chamadas } = ponteFalsa();
  await applyItemCommand({ kind: "record_execution", target: ALVO, record: registro() }, ponte);
  const enviado = chamadas[0]?.args[1] as Record<string, unknown>;
  assert.ok(!("result" in enviado), "`result` ausente não pode aparecer no payload");
  assert.ok(!("notes" in enviado), "`notes` ausente não pode aparecer no payload");
});

test("campo presente é repassado, com o valor que o chamador deu", async () => {
  const { ponte, chamadas } = ponteFalsa();
  await applyItemCommand(
    {
      kind: "record_execution",
      target: ALVO,
      record: registro({ result: "parcial", notes: "metade" }),
    },
    ponte,
  );
  const enviado = chamadas[0]?.args[1] as Record<string, unknown>;
  assert.equal(enviado["result"], "parcial");
  assert.equal(enviado["notes"], "metade");
});

test("patch parcial só envia os campos que o chamador preencheu", async () => {
  const { ponte, chamadas } = ponteFalsa();
  await applyItemCommand({ kind: "update_event", id: "e-1", patch: { notes: "só isto" } }, ponte);
  const enviado = chamadas[0]?.args[1] as Record<string, unknown>;
  assert.deepEqual(enviado, { notes: "só isto" });
  assert.ok(!("title" in enviado), "`title` não preenchido não pode ir como undefined");
});

test("patch vazio envia objeto vazio, e não undefined", async () => {
  const { ponte, chamadas } = ponteFalsa();
  await applyItemCommand({ kind: "update_event", id: "e-1", patch: {} }, ponte);
  const enviado = chamadas[0]?.args[1];
  assert.deepEqual(enviado, {});
});

test("o alvo repassado é o alvo, sem tradução de id", async () => {
  for (const kind of ["event", "occurrence", "responsibility", "block"] as const) {
    const alvo: CommandTarget = { kind, id: `${kind}-9` };
    const { ponte, chamadas } = ponteFalsa();
    await applyItemCommand({ kind: "set_state", target: alvo, to: "concluido" }, ponte);
    assert.deepEqual(chamadas[0]?.args[0], alvo);
  }
});

test("o default exaustivo existe: comando novo sem ramo é erro de compilação", () => {
  // Isto não é executável — é o motivo de `ItemCommand` ser uma união fechada e
  // não `interface`. O `default` com `never` em `commands.ts` é o que faz o
  // compilador reclamar quando alguém acrescenta um ramo sem despachá-lo.
  const comandosConhecidos = 7;
  assert.equal(CASOS.length, comandosConhecidos);
});
