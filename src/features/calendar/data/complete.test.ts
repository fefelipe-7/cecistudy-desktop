import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { completeGridItem, executionOf, type CompleteDeps } from "./complete.ts";
import type { ExecutionInput, ExecutionTargetWire, GridItem } from "./bridge.ts";
import { instant, minutes } from "../domain/time.ts";

/**
 * Contrato do concluir (INV-3 · D1.1).
 *
 * O que se prova aqui é o que a tela manda: um `execution_record` **e** um alvo
 * de ocorrência para `complete_item`. O Rust ainda é quem recusa sem o registro;
 * este teste prova que a tela não é o lado que omite o histórico.
 */

function item(): GridItem {
  return {
    id: "occ-1" as GridItem["id"],
    eventId: "evt-1" as GridItem["eventId"],
    layerId: "faculdade",
    title: "Aula de Cálculo I",
    startsAt: instant(0),
    endsAt: instant(3_600_000),
    state: "planejado",
    overrideKind: "none",
    isRecurring: false,
    hasConflict: false,
    origin: "cecistudy",
    allowedTransitions: ["concluido"],
  };
}

describe("concluir (INV-3 · D1.1)", () => {
  it("manda um execution_record para complete_item, e não um patch de estado", async () => {
    const chamadas: Array<{ target: ExecutionTargetWire; record: ExecutionInput }> = [];
    const deps: CompleteDeps = {
      completeItem: (target, record) => {
        chamadas.push({ target, record });
        return Promise.resolve();
      },
    };

    await completeGridItem(deps, item(), instant(30 * 60_000));

    assert.equal(chamadas.length, 1, "uma escrita, e é a do complete_item");
    assert.deepEqual(chamadas[0]?.target, { kind: "occurrence", id: "occ-1" });
    assert.ok(chamadas[0]?.record, "a escrita carrega o registro de execução");
  });

  it("o registro carrega resultado, origem e o instante do item", () => {
    const record = executionOf(item(), instant(30 * 60_000));
    assert.equal(record.result, "concluido");
    assert.equal(record.source, "manual");
    assert.equal(record.startedAt, instant(0));
    assert.equal(record.finishedAt, instant(30 * 60_000));
    assert.equal(record.actualDuration, minutes(30));
  });

  it("concluir antes do início não grava duração zero", () => {
    const record = executionOf(item(), instant(0));
    assert.equal(record.actualDuration, minutes(1), "duração mínima de 1 minuto");
  });
});
