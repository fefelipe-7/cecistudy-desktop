/**
 * Cache de leitura da janela do Calendário (DP1 · DP10).
 *
 * O defeito que este arquivo existe para remover: voltar no tempo custava uma
 * leitura completa. A tela tinha só `useState` e um efeito, então trocar de
 * `agenda` para `semana` e voltar disparava `list_events_in_window` de novo — e
 * essa leitura, no Rust, hoje **materializa as recorrências da janela inteira**
 * (`commands.rs:191-270`). Duas voltas no tempo, duas materializações, sem que
 * nada tivesse mudado no banco.
 *
 * Quatro decisões, e a última é a que evita a abstração errada:
 *
 * 1. **Chave é a janela, não a tela.** `Map<"from:to", GridItem[]>`.
 *
 *    Divergência declarada em relação ao texto da `DP1`, que previa
 *    `"<mode>:<from>:<to>"`: dois modos que caem na mesma janela compartilham a
 *    entrada, e é isso que se quer. `lista` e `planejamento` produzem a mesma
 *    janela e precisam das mesmas responsabilidades; `agenda` e `semana`
 *    divergem em janela, e então nem se encontram. O modo decide **se** a
 *    responsabilidade é lida (item 3), não **com qual chave** o evento é
 *    guardado. Incluir o modo na chave faria a segunda visita a `lista` custar
 *    um `invoke` que não precisava.
 * 2. **Escrita invalida; leitura não.** Uma releitura só acontece depois de uma
 *    escrita. É a única invalidação que importa aqui, e é a que o Rust entende.
 * 3. **Responsabilidades têm cache próprio.** A consulta é sem filtro, sem janela
 *    e — no Rust hoje — sem `LIMIT` (`commands.rs:550-556`): é a mais cara do
 *    arquivo, e três das cinco visões não mostravam uma linha do resultado. Ela
 *    é lida **só** nos modos que a exibem, e só uma vez.
 * 4. **Não é store de estado global.** Não é contexto React, não expõe
 *    `useSyncExternalStore`, não tem `subscribe`. Tem um consumidor — a tela — e
 *    um evento para consumir por enquanto. `useSyncExternalStore` entra com a
 *    spec 05 (sincronização por evento do Google), e nessa hora o `getSnapshot`
 *    precisa ser **estável por referência**, que é requisito documentado da API e
 *    não convenção.
 *
 * Testável com `node --test` sem Rust e sem Tauri, porque o leitor é injetado.
 */

import { listEventsInWindow, listResponsibilities, type GridItem } from "./bridge.ts";
import type { Instant, Responsibility, Window } from "../domain/types.ts";

/** Modos que precisam de responsabilidades para mostrar alguma coisa. */
const MODES_WITH_RESPONSIBILITIES: ReadonlySet<string> = new Set(["lista", "planejamento"]);

/** O que a tela precisa saber para renderizar um modo. */
export interface CalendarData {
  readonly items: readonly GridItem[];
  readonly responsibilities: readonly Responsibility[];
}

export interface CalendarQuery {
  readonly mode: string;
  readonly window: Window;
}

/**
 * As duas leituras, injetadas.
 *
 * Não é abstrair pelo abstrato: é o que torna este módulo verificável sem o
 * shell Tauri, que é o mesmo motivo de `*_core` existir em `commands.rs`.
 */
export interface CalendarReader {
  readonly listEventsInWindow: (window: Window) => Promise<readonly GridItem[]>;
  readonly listResponsibilities: () => Promise<readonly Responsibility[]>;
}

export interface CalendarStore {
  read(query: CalendarQuery): Promise<CalendarData>;
  /** Invalida o que a escrita tocou. Sem argumento, invalida tudo. */
  invalidate(scope?: "events" | "responsibilities" | "all"): void;
  /** Recarga explícita. */
  clear(): void;
  /** Quantas entradas estão vivas — usado nos testes e nos critérios de aceite. */
  readonly size: () => number;
}

const NO_ITEMS: readonly GridItem[] = [];
const NO_RESPONSIBILITIES: readonly Responsibility[] = [];

function windowKey(from: Instant, to: Instant): string {
  return `${from}:${to}`;
}

export function createCalendarStore(reader: CalendarReader): CalendarStore {
  const events = new Map<string, readonly GridItem[]>();
  let responsibilities: readonly Responsibility[] | null = null;

  /**
   * Duas leituras concorrentes da **mesma** janela compartilham **uma** chamada.
   *
   * Isto remove a duplicata, não a desordem. Trocar de modo produz janelas
   * **diferentes**, logo chaves diferentes, logo duas leituras realmente
   * concorrentes — e se a mais antiga resolver depois, quem escreve por último é
   * a janela que a usuária já deixou. A docstring deste arquivo dizia antes que o
   * cache resolvia isso e que o `cancelled` da tela "sumia junto com o cache".
   * **Não era verdade**, e o teste `calendar-store.test.ts` existe para manter
   * a afirmação falsa fora daqui: o caso "janelas diferentes resolvem fora de
   * ordem" está lá, e é um teste que **passa**, porque descreve o limite real.
   *
   * Quem garante a ordem é o chamador, por geração crescente
   * (`DP11` em `docs/performance/01-performance-render.md`). Cache e guarda de
   * ordenação são duas coisas diferentes, e as duas são necessárias.
   *
   * Vive **dentro** da factory: um mapa no escopo do módulo seria compartilhado
   * entre duas instâncias, e a deduplicação de uma store responderia por uma
   * leitura da outra.
   */
  const inFlightEvents = new Map<string, Promise<readonly GridItem[]>>();
  let inFlightResponsibilities: Promise<readonly Responsibility[]> | null = null;

  function readEvents(window: Window): Promise<readonly GridItem[]> {
    const key = windowKey(window.from, window.to);
    const cached = events.get(key);
    if (cached !== undefined) return Promise.resolve(cached);

    let pending = inFlightEvents.get(key);
    if (pending === undefined) {
      pending = reader.listEventsInWindow(window);
      inFlightEvents.set(key, pending);
      const release = (): void => {
        inFlightEvents.delete(key);
      };
      pending.then(release, release);
    }

    return pending.then((value) => {
      // Só grava depois de resolver: falha não entra no cache, senão o próximo
      // `read` devolveria o erro pelo resto da sessão.
      events.set(key, value);
      return value;
    });
  }

  function readResponsibilities(): Promise<readonly Responsibility[]> {
    if (responsibilities !== null) return Promise.resolve(responsibilities);

    if (inFlightResponsibilities === null) {
      const request = reader.listResponsibilities();
      inFlightResponsibilities = request;
      const release = (): void => {
        inFlightResponsibilities = null;
      };
      request.then(release, release);
    }

    return inFlightResponsibilities.then((value) => {
      responsibilities = value;
      return value;
    });
  }

  return {
    read(query: CalendarQuery): Promise<CalendarData> {
      const items = readEvents(query.window);

      // `lista` e `planejamento` são os únicos modos que mostram responsabilidade.
      // Nos outros três, a promessa de responsabilidade **não chega a ser criada**
      // — chamá-la e descartar o resultado seria pior do que não chamar, porque o
      // `listResponsibilities` do Rust já teria sido disparado.
      if (!MODES_WITH_RESPONSIBILITIES.has(query.mode)) {
        return items.then((loaded) => ({ items: loaded, responsibilities: NO_RESPONSIBILITIES }));
      }

      return Promise.all([items, readResponsibilities()]).then(([loadedItems, loadedPending]) => ({
        items: loadedItems,
        responsibilities: loadedPending,
      }));
    },

    invalidate(scope = "all"): void {
      if (scope === "all" || scope === "events") events.clear();
      if (scope === "all" || scope === "responsibilities") responsibilities = null;
    },

    clear(): void {
      events.clear();
      responsibilities = null;
    },

    size(): number {
      return events.size + (responsibilities === null ? 0 : 1);
    },
  };
}

/** Atalho interno para a tela não precisar repetir a lista vazia. */
export { NO_ITEMS, NO_RESPONSIBILITIES };

/**
 * A instância que a tela usa (`DP11`).
 *
 * Singleton de **módulo**, e não de contexto React: o cache precisa sobreviver
 * à troca de janela e ao remontar da tela, e o `DP10` já recusou
 * `useSyncExternalStore` por enquanto. Uma instância por tela perderia tudo o
 * que o cache existe para guardar.
 *
 * O leitor é o `bridge` de verdade, e não um dublê: assim a tela não tem como
 * ler por fora do cache sem passar por ele.
 */
export const calendarStore: CalendarStore = createCalendarStore({
  listEventsInWindow: (window) => listEventsInWindow(window),
  listResponsibilities: () => listResponsibilities(),
});
