/**
 * Contrato de `item-menu.ts` (`02-grade-desktop.md` Fase 2.5).
 *
 * Os casos abaixo são os que a spec nomeia, mais os que a tabela de §3.1
 * obriga. O ponto que mais importa: **nenhum destes testes prova que a
 * transição é legal** — quem prova é o Rust (`transicao_ilegal_e_recusada_na_
 * escrita`). Aqui o que se prova é que o menu não oferece botão que volta erro.
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { actionAllowed, actionsFor, requiresRecurrenceScope } from "./item-menu.ts";
import type { MenuContext } from "./item-menu.ts";
import type { ItemState, Origin } from "./types.ts";

const ctx = (over: Partial<MenuContext> = {}): MenuContext => ({
  origin: "cecistudy" as Origin,
  state: "planejado" as ItemState,
  ...over,
});

describe("origem google: somente leitura (INV-6)", () => {
  it("não oferece editar, arrastar, redimensionar nem excluir", () => {
    const c = ctx({ origin: "google" });
    for (const acao of ["editar", "arrastar", "redimensionar", "excluir"] as const) {
      assert.equal(actionAllowed(acao, c), false, `google não pode "${acao}"`);
    }
  });

  it("oferece registrar execução — §10: prepares-se a reunião sem editá-la", () => {
    const c = ctx({ origin: "google" });
    assert.equal(actionAllowed("registrar_execucao", c), true);
    assert.deepEqual(actionsFor(c), ["registrar_execucao"]);
  });

  it("o menu inteiro fica em uma ação só", () => {
    assert.equal(actionsFor(ctx({ origin: "google" })).length, 1);
  });

  it("google nunca conclui o evento de terceiro", () => {
    assert.equal(actionAllowed("concluir", ctx({ origin: "google" })), false);
  });
});

describe("estado encerrado: cancelado e dispensado", () => {
  it("cancelado não edita, mas reativa", () => {
    const c = ctx({ state: "cancelado" });
    assert.equal(actionAllowed("editar", c), false);
    assert.equal(actionAllowed("reativar", c), true, "§4: cancelado volta para planejado");
    assert.equal(actionAllowed("concluir", c), false);
  });

  it("dispensado não edita, mas reativa", () => {
    const c = ctx({ state: "dispensado" });
    assert.equal(actionAllowed("editar", c), false);
    assert.equal(actionAllowed("reativar", c), true, "§4: dispensado é decisão revisável");
  });

  it("não oferece arrastar nem redimensionar em estado encerrado", () => {
    for (const state of ["cancelado", "dispensado"] as const) {
      const c = ctx({ state });
      assert.equal(actionAllowed("arrastar", c), false);
      assert.equal(actionAllowed("redimensionar", c), false);
    }
  });
});

describe("concluido tem uma única saída", () => {
  it("oferece reabrir e não oferece concluir de novo", () => {
    const c = ctx({ state: "concluido" });
    assert.equal(actionAllowed("reabrir", c), true, "concluido → em_andamento");
    assert.equal(actionAllowed("concluir", c), false);
  });

  it("não oferece reativar, porque a tabela não tem concluido → planejado", () => {
    assert.equal(actionAllowed("reativar", ctx({ state: "concluido" })), false);
  });
});

describe("o menu não oferece o que a tabela de §3.1 recusa", () => {
  it("planejado oferece concluir", () => {
    assert.equal(actionAllowed("concluir", ctx({ state: "planejado" })), true);
  });

  it("adiado oferece concluir e reativar", () => {
    const c = ctx({ state: "adiado" });
    assert.equal(actionAllowed("concluir", c), true);
    assert.equal(actionAllowed("reativar", c), true);
  });

  it("em_andamento não oferece concluir... oferece, mas não reativar", () => {
    const c = ctx({ state: "em_andamento" });
    assert.equal(actionAllowed("concluir", c), true);
    // `em_andamento → planejado` não está na tabela.
    assert.equal(actionAllowed("reativar", c), false);
  });

  it("nenhum estado oferece concluir quando a tabela não permite", () => {
    const estados: ItemState[] = [
      "planejado",
      "em_andamento",
      "concluido",
      "adiado",
      "nao_realizado",
      "cancelado",
      "dispensado",
    ];
    for (const state of estados) {
      const oferece = actionAllowed("concluir", ctx({ state }));
      const tabelaPermite =
        state !== "concluido" && state !== "cancelado" && state !== "dispensado";
      assert.equal(oferece, tabelaPermite, `${state}: o menu e a tabela de §3.1 discordam`);
    }
  });
});

describe("recorrência: §4 exige escopo antes de qualquer mudança", () => {
  it("ocorrência com regra pede escopo", () => {
    assert.equal(requiresRecurrenceScope(ctx({ hasRecurrenceRule: true })), true);
  });

  it("item avulso não pede escopo", () => {
    assert.equal(requiresRecurrenceScope(ctx({ hasRecurrenceRule: false })), false);
    assert.equal(requiresRecurrenceScope(ctx()), false);
  });

  it("pedir escopo não esconde as ações: elas continuam, com escopo", () => {
    const c = ctx({ hasRecurrenceRule: true });
    assert.equal(actionAllowed("editar", c), true);
    assert.equal(actionAllowed("arrastar", c), true);
  });
});

describe("forma da saída", () => {
  it("devolve a lista completa de um item planejado e comum", () => {
    assert.deepEqual(actionsFor(ctx()), [
      "editar",
      "arrastar",
      "redimensionar",
      "registrar_execucao",
      "concluir",
      "excluir",
    ]);
  });

  it("a ordem é estável entre chamadas", () => {
    assert.deepEqual(actionsFor(ctx()), actionsFor(ctx()));
  });

  it("isReadOnlyOrigin sobrepõe a origem, para teste e para origem derivada", () => {
    const c = ctx({ origin: "cecistudy", isReadOnlyOrigin: true });
    assert.deepEqual(actionsFor(c), ["registrar_execucao"]);
  });
});
