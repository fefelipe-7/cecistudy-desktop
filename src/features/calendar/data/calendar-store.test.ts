import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { createCalendarStore, type CalendarData, type CalendarReader } from "./calendar-store.ts";
import type { GridItem } from "./bridge.ts";
import type { Instant, Responsibility, Window } from "../domain/types.ts";
import { instant } from "../domain/time.ts";
import { responsibilityId } from "../domain/types.ts";

/**
 * Contrato do cache de janela (`DP1` · `DP10` · `DP11`).
 *
 * O leitor é injetado, então isto roda em `node --test` sem Rust, sem Tauri e
 * sem DOM — que é a razão de o módulo existir separado da tela.
 *
 * O teste que mais importa é o último. Ele documenta um limite do cache: a
 * deduplicação cobre **a mesma janela**, e não a ordem entre janelas
 * diferentes. Escrever a tela contra a promessa errada da docformer é o defeito
 * que o `DP11` precisou desfazer.
 */

/** Promessa que resolve quando o teste mandar, para controlar a ordem. */
function defer<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

function janela(from: number, to: number): Window {
  return { from: instant(from), to: instant(to) };
}

function item(id: string): GridItem {
  return {
    id: id as GridItem["id"],
    eventId: `evento-${id}` as GridItem["eventId"],
    layerId: "faculdade",
    title: `Item ${id}`,
    startsAt: instant(0),
    endsAt: instant(3_600_000),
    state: "planejado",
    overrideKind: "none",
    isRecurring: false,
    hasConflict: false,
    origin: "cecistudy",
    allowedTransitions: [],
  };
}

function responsabilidade(id: string): Responsibility {
  return {
    id: responsibilityId(id),
    createdAt: instant(0),
    updatedAt: instant(0),
    layerId: "faculdade",
    title: `Responsabilidade ${id}`,
    kind: "tarefa",
    commitment: "importante",
    state: "planejado",
    dueAt: instant(86_400_000),
    plannedDuration: null,
    origin: "cecistudy",
    parentId: null,
    isOverdue: false,
  };
}

/** Contador de chamadas, para provar que o cache não deixou passar nenhuma. */
function leitor(opcoes: {
  readonly eventos?: (w: Window) => Promise<readonly GridItem[]>;
  readonly responsabilidades?: () => Promise<readonly Responsibility[]>;
}): CalendarReader & { readonly nEventos: number; readonly nResponsabilidades: number } {
  let nEventos = 0;
  let nResponsabilidades = 0;
  return {
    get nEventos() {
      return nEventos;
    },
    get nResponsabilidades() {
      return nResponsabilidades;
    },
    listEventsInWindow(w) {
      nEventos += 1;
      return opcoes.eventos ? opcoes.eventos(w) : Promise.resolve([item(`w${nEventos}`)]);
    },
    listResponsibilities() {
      nResponsabilidades += 1;
      return opcoes.responsabilidades
        ? opcoes.responsabilidades()
        : Promise.resolve([responsabilidade(`r${nResponsabilidades}`)]);
    },
  };
}

describe("calendar-store · cache por janela", () => {
  it("a segunda leitura da mesma janela não chama o leitor", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    const a = await store.read({ mode: "semana", window: w });
    const b = await store.read({ mode: "semana", window: w });

    assert.equal(r.nEventos, 1, "a releitura da mesma janela custou um invoke");
    assert.equal(a.items, b.items, "o cache precisa devolver a mesma referência");
  });

  it("janelas diferentes são entradas diferentes", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);

    await store.read({ mode: "semana", window: janela(0, 10) });
    await store.read({ mode: "semana", window: janela(0, 20) });
    await store.read({ mode: "semana", window: janela(0, 10) });

    assert.equal(r.nEventos, 2, "a janela repetida tem de vir do cache");
    assert.equal(store.size(), 2);
  });

  it("voltar no tempo não custa leitura", async () => {
    // Este é o defeito que o módulo existe para remover: `agenda` -> `semana` ->
    // `agenda` disparava duas materializações no Rust sem que o banco mudasse.
    const r = leitor({});
    const store = createCalendarStore(r);
    const semana = janela(0, 10);
    const agenda = janela(0, 4);

    await store.read({ mode: "semana", window: semana });
    await store.read({ mode: "agenda", window: agenda });
    const volta = await store.read({ mode: "agenda", window: agenda });

    assert.equal(r.nEventos, 2);
    assert.equal(volta.items.length, 1);
  });
});

describe("calendar-store · responsabilidades por modo", () => {
  it("modos que não mostram responsabilidade não disparam a consulta", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    // A consulta do Rust é sem filtro, sem janela e sem `LIMIT`: é a mais cara
    // do arquivo, e `semana`, `mes` e `agenda` não mostram uma linha dela.
    for (const mode of ["semana", "mes", "agenda"]) {
      const data = await store.read({ mode, window: w });
      assert.equal(data.responsibilities.length, 0);
    }
    assert.equal(r.nResponsabilidades, 0, "chamar e descartar seria pior do que não chamar");
  });

  it("`lista` e `planejamento` leem, e a leitura é aproveitada", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    await store.read({ mode: "lista", window: w });
    await store.read({ mode: "planejamento", window: w });

    assert.equal(r.nResponsabilidades, 1);
  });
});

describe("calendar-store · invalidação", () => {
  it("`invalidate('events')` preserva as responsabilidades", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    await store.read({ mode: "lista", window: w });
    store.invalidate("events");

    const depois = await store.read({ mode: "lista", window: w });
    assert.equal(r.nEventos, 2, "eventos precisam ter sido relidos");
    assert.equal(r.nResponsabilidades, 1, "responsabilidade não foi tocada");
    assert.equal(depois.responsibilities.length, 1);
  });

  it("`invalidate('all')` e `clear()` limpam tudo", async () => {
    const r = leitor({});
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    await store.read({ mode: "lista", window: w });
    store.invalidate("all");
    assert.equal(store.size(), 0);

    await store.read({ mode: "lista", window: w });
    store.clear();
    assert.equal(store.size(), 0);
  });
});

describe("calendar-store · concorrência", () => {
  it("duas leituras concorrentes da MESMA janela compartilham uma chamada", async () => {
    const pendente = defer<readonly GridItem[]>();
    const r = leitor({ eventos: () => pendente.promise });
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    const primeira = store.read({ mode: "semana", window: w });
    const segunda = store.read({ mode: "semana", window: w });
    pendente.resolve([item("unico")]);

    const [a, b] = await Promise.all([primeira, segunda]);
    assert.equal(r.nEventos, 1, "duas chamadas concorrentes viraram duas leituras");
    assert.deepEqual(a.items, b.items);
  });

  it("falha não entra no cache", async () => {
    let tentativa = 0;
    const r = leitor({
      eventos: () => {
        tentativa += 1;
        return tentativa === 1
          ? Promise.reject(new Error("invoke falhou"))
          : Promise.resolve([item("ok")]);
      },
    });
    const store = createCalendarStore(r);
    const w = janela(0, 10);

    await assert.rejects(store.read({ mode: "semana", window: w }));
    const segunda = await store.read({ mode: "semana", window: w });

    assert.equal(r.nEventos, 2);
    assert.equal(segunda.items.length, 1, "o erro não pode ficar no cache pelo resto da sessão");
  });
});

describe("calendar-store · o limite que o cache NÃO cobre", () => {
  it("janelas diferentes em voo resolvem fora de ordem, e o cache não protege", async () => {
    // Este teste é a razão de `DP11` não ter removido o guarda de ordenação.
    //
    // A deduplicação em voo é por **chave de janela**. Trocar de modo produz
    // janelas diferentes, logo duas leituras realmente concorrentes — e se a mais
    // antiga resolver depois, quem escreve por último é a janela velha.
    // `calendar-store.ts` não resolve isso, e a docformer dizia que resolvia.
    // A correção é do chamador (geração crescente em `CalendarScreen`), e este
    // teste existe para impedir que a docformer volte a prometer o cache.
    const antiga = defer<readonly GridItem[]>();
    const nova = defer<readonly GridItem[]>();
    let chamada = 0;
    const r = leitor({
      eventos: () => {
        chamada += 1;
        return chamada === 1 ? antiga.promise : nova.promise;
      },
    });
    const store = createCalendarStore(r);

    const leituraAntiga = store.read({ mode: "semana", window: janela(0, 10) });
    const leituraNova = store.read({ mode: "agenda", window: janela(0, 4) });

    // A janela nova resolve primeiro; a velha depois. As duas promessas são
    // válidas e o cache não escolhe: quem decide é quem escreve.
    nova.resolve([item("nova")]);
    const dadosNovos = await leituraNova;
    antiga.resolve([item("antiga")]);
    const dadosAntigos = await leituraAntiga;

    assert.equal(r.nEventos, 2, "as duas janelas são leituras distintas, como esperado");
    assert.equal(dadosNovos.items[0]?.title, "Item nova");
    assert.equal(
      dadosAntigos.items[0]?.title,
      "Item antiga",
      "o cache devolve a leitura velha por último: quem tem de descartar é o chamador",
    );
  });
});
