/**
 * O golden é o contrato entre os dois lados da regra (D61 da SPEC-D-005).
 *
 * Este arquivo lê o **mesmo** `golden/recurrence.jsonl` que o teste Rust em
 * `src-tauri/src/commands.rs` lê, e exercita o **código real do espelho** —
 * `expandRule` de `recurrence.ts` — e não uma reimplementação. Um golden
 * verificado contra uma cópia da regra não prova nada sobre a regra que a UI
 * usa.
 *
 * O `dtstartTz` é `UTC` de propósito: o golden fixa instantes em dias a partir
 * da âncora, e a conversão de parede só seria um segundo eixo de divergência
 * além do que se quer medir aqui. A conversão de fuso tem suíte própria em
 * `time.test.ts`.
 *
 * Rust é a autoridade (ADR-006, corolário 2). Este lado é espelho: existe para a
 * render e para rodar teste sem o toolchain, e é o lado errado quando os dois
 * discordam.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { expandRule } from "./recurrence.ts";
import { instant } from "./time.ts";
import type { CalendarEventId, IanaTimeZone, RecurrenceRuleId } from "./types.ts";

const GOLDEN = fileURLToPath(new URL("../../../../golden/recurrence.jsonl", import.meta.url));

const DIA = 86_400_000;
const HORA = 3_600_000;

interface Caso {
  readonly caso: string;
  readonly entrada: {
    readonly rrule: string;
    readonly ancora: string;
    readonly meses?: number;
    readonly dias?: number;
    readonly anos?: number;
    readonly count?: number;
  };
  readonly saida: { readonly diasDaAncora: readonly number[] };
}

function linhas(): readonly Caso[] {
  return readFileSync(GOLDEN, "utf8")
    .split(/\r?\n/)
    .map((linha) => linha.trim())
    .filter((linha) => linha.length > 0 && !linha.startsWith("#"))
    .map((linha) => JSON.parse(linha) as Caso);
}

/** ISO `YYYY-MM-DDTHH:MM:SSZ` para epoch ms, sem `Date.parse` opaco. */
function isoParaMs(iso: string): number {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})Z$/.exec(iso);
  if (!m) throw new Error(`ISO fora do formato do golden: ${iso}`);
  // Grupos 1..6: ano, mês, dia, hora, minuto, segundo. A ordem é do regex, e
  // desestruturar por posição erra silenciosamente — `data` viraria só o ano.
  return Date.UTC(+m[1]!, +m[2]! - 1, +m[3]!, +m[4]!, +m[5]!, +m[6]!);
}

test("o espelho TypeScript concorda com o golden de recorrência", () => {
  const lista = linhas();
  for (const caso of lista) {
    const ancora = isoParaMs(caso.entrada.ancora);
    const esperado = [...caso.saida.diasDaAncora];

    // A janela é larga e `count` vem do próprio golden, que é o limite que o
    // Rust também recebe. Sem os dois, os lados geram números diferentes de
    // ocorrências e a comparação seria sobre coisas distintas.
    const drafts = expandRule(
      { id: "evt-1" as CalendarEventId, startsAt: instant(ancora), endsAt: instant(ancora + HORA) },
      {
        id: "rule-1" as RecurrenceRuleId,
        eventId: "evt-1" as CalendarEventId,
        rrule: caso.entrada.rrule,
        dtstartTz: "UTC" as IanaTimeZone,
        count: esperado.length,
        exdates: [],
        // `RecurrenceRule` é auditável; o golden não testa auditoria e os
        // valores são ancorados na própria âncora para não introduzir um eixo
        // de divergência que não é o que esta prova.
        createdAt: instant(ancora),
        updatedAt: instant(ancora),
      },
      { from: instant(ancora), to: instant(ancora + 4000 * DIA) },
    );

    const obtido = drafts.map((d) => (d.originalStart - ancora) / DIA);
    assert.deepEqual(
      obtido,
      esperado,
      `caso \`${caso.caso}\`: o espelho TypeScript discorda do golden`,
    );
  }
  assert.equal(lista.length, 12);
});

test("o golden de recorrência cobre as capacidades que o parser antigo não tinha", () => {
  // Se alguém trocar o motor por um subconjunto, o golden encolhe e este teste
  // falha — que é a proteção contra a regressão que a C6 descrevia.
  const casos = linhas().map((c) => c.caso);
  for (const exigido of [
    "monthly-bysetpos-ultima-sexta",
    "monthly-byday-1mo-primeira-segunda",
    "monthly-bymonthday-31",
    "yearly-mesmo-dia-do-ano",
  ]) {
    assert.ok(casos.includes(exigido), `o golden perdeu o caso \`${exigido}\``);
  }
});
