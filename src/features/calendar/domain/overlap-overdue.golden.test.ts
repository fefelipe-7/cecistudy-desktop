/**
 * O golden é o contrato entre os dois lados da regra (D61 da SPEC-D-005).
 *
 * Este arquivo lê os **mesmos** `golden/overlap.jsonl` e `golden/overdue.jsonl`
 * que os testes Rust em `src-tauri/src/domain.rs` lêem. Divergência entre os dois
 * lados é falha, e a falha nomeia o caso.
 *
 * Rust é a autoridade (ADR-006, corolário 2). Este lado é espelho: existe para a
 * render e para rodar teste sem o toolchain, e é o lado errado quando os dois
 * discordam.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { findOverlaps } from "./overlap.ts";
import { isOverdue } from "./overdue.ts";
import { addMinutes, instant, minutes } from "./time.ts";
import type { Commitment, ItemState } from "./types.ts";

const GOLDEN = (nome: string): string =>
  fileURLToPath(new URL(`../../../../golden/${nome}`, import.meta.url));

const HORA = 3_600_000;

interface Item {
  readonly id: string;
  readonly inicioHora: number;
  readonly fimHora: number;
}

function linhas(nome: string): readonly string[] {
  return readFileSync(GOLDEN(nome), "utf8")
    .split(/\r?\n/)
    .map((linha) => linha.trim())
    .filter((linha) => linha.length > 0 && !linha.startsWith("#"));
}

test("D7 · o espelho TypeScript concorda com o golden de sobreposição", () => {
  const lista = linhas("overlap.jsonl");
  for (const linha of lista) {
    const caso = JSON.parse(linha) as {
      readonly caso: string;
      readonly entrada: { readonly items: readonly Item[] };
      readonly saida: { readonly emConflito: Readonly<Record<string, boolean>> };
    };

    // Mesmo caminho do espelho: `findOverlaps` devolve grupos, e todo item de
    // um grupo está em conflito. Item isolado não aparece em grupo nenhum.
    //
    // `Instant` é branded, então os valores passam pelos construtores em vez
    // de `as never`: um cast esconderia o erro de que o golden devolve número
    // cru, e é exatamente esse o bug que o branding existe para pegar.
    const caixas = caso.entrada.items.map((item) => {
      const startsAt = instant(Date.UTC(2026, 8, 29, item.inicioHora, 0, 0));
      return {
        id: item.id,
        startsAt,
        endsAt: addMinutes(startsAt, minutes((item.fimHora - item.inicioHora) * 60)),
      };
    });
    const marcados = new Set<string>();
    for (const [ancora, resto] of findOverlaps(caixas)) {
      marcados.add(ancora.id);
      for (const item of resto) marcados.add(item.id);
    }

    for (const item of caso.entrada.items) {
      assert.equal(
        marcados.has(item.id),
        caso.saida.emConflito[item.id],
        `caso \`${caso.caso}\`, item \`${item.id}\`: o espelho discorda do golden`,
      );
    }
  }
  assert.equal(lista.length, 10);
});

test("INV-4 · o espelho TypeScript concorda com o golden de atraso", () => {
  const lista = linhas("overdue.jsonl");
  for (const linha of lista) {
    const caso = JSON.parse(linha) as {
      readonly caso: string;
      readonly entrada: {
        readonly compromisso: Commitment;
        readonly estado: ItemState;
        readonly dueAt: number | null;
        readonly agoraHoras: number;
      };
      readonly saida: { readonly atrasado: boolean };
    };

    const obtido = isOverdue(
      {
        commitment: caso.entrada.compromisso,
        state: caso.entrada.estado,
        dueAt: caso.entrada.dueAt === null ? null : instant(caso.entrada.dueAt),
      },
      instant(caso.entrada.agoraHoras * HORA),
    );

    assert.equal(
      obtido,
      caso.saida.atrasado,
      `caso \`${caso.caso}\`: o espelho discorda do golden`,
    );
  }
  // 4 compromissos × 7 estados × 3 prazos
  assert.equal(lista.length, 84);
});
