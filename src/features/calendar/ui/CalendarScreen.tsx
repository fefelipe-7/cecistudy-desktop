/**
 * Tela do Calendário (spec 01 Fase 1.8 · estrutura de §11 de `calendario.md`).
 *
 * O Calendário é **uma tela só**, com cinco modos — não cinco destinos. Não há
 * roteador: o modo é `useState`, como o resto do app (`AGENTS.md`).
 *
 * - **Semana** — modo inicial no desktop. Dias no eixo horizontal, horas no
 *   vertical. É a leitura principal: é onde o planejamento e a realidade se
 *   alinham.
 * - **Agenda** — o mesmo dia decomposto em lista, para quem trabalha por itens e
 *   não por posição no tempo.
 * - **Mês** — visão global; aqui entram os prazos que não ocupam espaço.
 * - **Lista** — o que precisa ser feito, agrupado em "Hoje" e "Próximos dias".
 * - **Planejamento** — o que foi reservado contra o que foi executado.
 *
 * O painel da direita **não é uma tela separada**: ele é o detalhe do item
 * selecionado, e some quando nada está selecionado. Por isso vive dentro deste
 * componente e não tem entrada no menu.
 *
 * O que este arquivo ainda NÃO faz (e é de propósito): arrastar, redimensionar,
 * criar e editar. A grade visual completa é a spec 02; aqui a tela carrega os
 * repositórios, decide o estado vazio e já separa as cinco visões.
 */

import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { CalendarDays, ChevronLeft, ChevronRight, ListChecks, Repeat } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { isIpcAvailable } from "@/lib/ipc";

import { listEventsInWindow, listResponsibilities, type GridItem } from "../data/bridge.ts";
import { seedIfEmpty } from "../data/dev-seed.ts";
import { isOpenState, stateLabel } from "../domain/commitment.ts";
import { indexByDay, itemsOn, type DayIndex } from "../domain/day-index.ts";
import { DEFAULT_SCALE, timeToY, visibleHeight, type GridScale } from "../domain/grid-scale.ts";
import { layerClass, layerTone } from "../domain/layers.ts";
import { isOverdue } from "../domain/overdue.ts";
import {
  addDaysInTz,
  daysInMonth,
  fromPlainDate,
  instant,
  plainDate,
  toPlainDate,
  type Instant,
} from "../domain/time.ts";
import type { IanaTimeZone, LayerId, Responsibility } from "../domain/types.ts";
import { CALENDAR_ZONE } from "../domain/zone.ts";
import { DayColumn } from "./DayColumn.tsx";
import {
  formatClock,
  formatLongDay,
  formatMonthYear,
  formatWeekdayLong,
  formatWeekdayShort,
  hourLabels,
} from "./format.ts";

/** Modos da tela. `semana` é o padrão porque é a leitura principal do desktop. */
export type CalendarMode = "agenda" | "semana" | "mes" | "lista" | "planejamento";

const MODES: ReadonlyArray<{ readonly id: CalendarMode; readonly label: string }> = [
  { id: "agenda", label: "Agenda" },
  { id: "semana", label: "Semana" },
  { id: "mes", label: "Mês" },
  { id: "lista", label: "Lista" },
  { id: "planejamento", label: "Planejamento" },
];

const ZONE = CALENDAR_ZONE;

/** Densidade da grade (D16). Uma escala só, compartilhada por toda a tela. */
const SCALE: GridScale = { ...DEFAULT_SCALE, timeZone: ZONE };

const WEEKDAY_LABELS = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"] as const;

function startOfWeek(now: Instant, zone: IanaTimeZone): Instant {
  // Segunda como primeiro dia: é a convenção de `calendario.md` §11.
  const today = plainDate(toPlainDate(now, zone));
  const weekday = new Date(`${today}T00:00:00Z`).getUTCDay();
  const offset = weekday === 0 ? -6 : 1 - weekday;
  return fromPlainDate(plainDate(shiftPlainDate(today, offset)), zone);
}

function shiftPlainDate(value: string, days: number): string {
  const date = new Date(`${value}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

export interface CalendarScreenProps {
  readonly onOpenList: () => void;
}

export function CalendarScreen({ onOpenList }: CalendarScreenProps) {
  // A hora de referência entra por `useMemo` sobre o estado de "agora", e não
  // por `Date.now()` espalhado: o domínio só conhece instantes.
  const [now, setNow] = useState<Instant>(() => instant(Date.now()));
  const [mode, setMode] = useState<CalendarMode>("semana");
  const [anchor, setAnchor] = useState<Instant>(() =>
    // A data civil vem do fuso **da grade**, não de `toISOString()`. Entre 21:00
    // e 00:00 no Brasil, `toISOString()` já devolve o dia seguinte e o app abre na
    // quinta-feira quando a usuária está na quinta — a nenhum teste isso aparece,
    // porque `Date` aqui é o relógio da máquina e o fuso também.
    fromPlainDate(toPlainDate(instant(Date.now()), ZONE), ZONE),
  );
  const [items, setItems] = useState<readonly GridItem[]>([]);
  const [responsibilities, setResponsibilities] = useState<readonly Responsibility[]>([]);
  const [selected, setSelected] = useState<GridItem | null>(null);
  const [loaded, setLoaded] = useState(false);
  // A semente de dev só é tentada uma vez por sessão, mesmo que a janela de
  // leitura mude várias vezes.
  const seedAttempted = useRef(false);

  // Relógio de 1 em 1 minuto: a grade precisa saber "agora" para marcar a linha
  // do momento e para o atraso. `Date` fica confinado a este ponto de entrada.
  useEffect(() => {
    const timer = setInterval(() => setNow(instant(Date.now())), 60_000);
    return () => clearInterval(timer);
  }, []);

  const weekStart = useMemo(() => startOfWeek(anchor, ZONE), [anchor]);

  /**
   * A janela carregada segue o modo, e não é sempre a semana.
   *
   * Um dia de folga em cada ponta porque um item que atravessa a meia-noite
   * começa no dia anterior — e `placeInDay` devolve o pedaço na coluna do dia
   * seguinte. Sem a folga, o pedaço nunca apareceria.
   */
  const window = useMemo(() => {
    if (mode === "mes") {
      // 6 semanas × 7 dias, a partir da semana do dia 1.
      const monthStart = fromPlainDate(
        plainDate(`${toPlainDate(anchor, ZONE).slice(0, 7)}-01`),
        ZONE,
      );
      const gridStart = startOfWeek(monthStart, ZONE);
      return { from: addDaysInTz(gridStart, -1, ZONE), to: addDaysInTz(gridStart, 43, ZONE) };
    }
    if (mode === "agenda") {
      return { from: addDaysInTz(anchor, -1, ZONE), to: addDaysInTz(anchor, 2, ZONE) };
    }
    if (mode === "lista" || mode === "planejamento") {
      // Estas visões agregam horizonte, não dia: precisam de uma janela larga.
      return { from: addDaysInTz(weekStart, -7, ZONE), to: addDaysInTz(weekStart, 60, ZONE) };
    }
    return { from: addDaysInTz(weekStart, -1, ZONE), to: addDaysInTz(weekStart, 8, ZONE) };
  }, [anchor, mode, weekStart]);

  useEffect(() => {
    let cancelled = false;
    // Sem shell Tauri não há backend: a tela mostra o estado vazio em vez de
    // quebrar. É o mesmo caminho do `npm run dev` no browser puro.
    if (!isIpcAvailable()) {
      setLoaded(true);
      return;
    }

    const load = async (): Promise<void> => {
      const [events, pending] = await Promise.all([
        listEventsInWindow(window),
        listResponsibilities(),
      ]);
      if (cancelled) return;
      setItems(events);
      setResponsibilities(pending);
    };

    void (async () => {
      try {
        await load();
        // A semente roda **depois** da primeira leitura, e só uma vez por sessão:
        // decidir antes seria decidir sobre um banco que ainda não foi lido, e
        // decidir a cada troca de janela gastaria uma consulta à toa.
        if (!cancelled && !seedAttempted.current && (await seedIfEmpty())) {
          seedAttempted.current = true;
          await load();
        }
      } catch {
        // Falha de leitura não pode derrubar a tela: o estado vazio já é uma
        // resposta honesta, e o erro aparece quando a usuária tenta agir.
      } finally {
        if (!cancelled) setLoaded(true);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [window]);

  const days = useMemo(
    () => Array.from({ length: 7 }, (_, index) => addDaysInTz(weekStart, index, ZONE)),
    [weekStart],
  );

  // O índice cobre a **janela inteira**, não só os dias visíveis: a Agenda
  // pergunta sobre um dia que pode estar fora da semana em foco, e responder
  // "não sei" seria o mesmo bug de antes com outro nome (A-09).
  const byDay = useMemo(() => indexByDay(items, window, ZONE), [items, window]);

  /**
   * O estado vazio é **por modo**, não global. Antes um `items.length === 0`
   * escondia a Lista e o Planejamento atrás de um convite a criar evento — mesmo
   * com dezenas de responsabilidades a perder, que é o conteúdo desses modos.
   */
  const empty =
    loaded &&
    (mode === "lista" || mode === "planejamento"
      ? items.length === 0 && responsibilities.length === 0
      : items.length === 0);

  // A Agenda é por **dia**, e dia é um estado próprio: antes ela recebia o início
  // da semana, então abria sempre na segunda-feira em vez de hoje.
  const step = (days_: number): void => setAnchor((current) => addDaysInTz(current, days_, ZONE));
  const stepMonth = (months: number): void =>
    setAnchor((current) => {
      const key = toPlainDate(current, ZONE);
      const year = Number(key.slice(0, 4));
      const month = Number(key.slice(5, 7)) - 1 + months;
      // `Date` resolve o mês em UTC; normalizar para o dia 1 evita o caso de
      // 31 de janeiro virar 3 de março ao somar um mês.
      const shifted = new Date(Date.UTC(year, month, 1));
      const next = plainDate(
        `${String(shifted.getUTCFullYear()).padStart(4, "0")}-${String(
          shifted.getUTCMonth() + 1,
        ).padStart(2, "0")}-01`,
      );
      return fromPlainDate(next, ZONE);
    });

  const goToday = (): void => setAnchor(fromPlainDate(plainDate(toPlainDate(now, ZONE)), ZONE));

  /**
   * Seleção **não** é estado da grade (DP6).
   *
   * Antes `setSelected` vinha pronto para os filhos, e o `memo` nunca acertava
   * porque a função era nova a cada render — mesmo com `items` idêntico. Com o
   * `useCallback`, selecionar um item não propaga para a Semana nenhuma, e é isso
   * que torna "1 render por clique" o comportamento em vez de uma esperança.
   */
  const select = useCallback((item: GridItem) => setSelected(item), []);
  const clearSelection = useCallback(() => setSelected(null), []);

  const title = useMemo(() => {
    if (mode === "mes") return formatMonthYear(anchor);
    if (mode === "semana" || mode === "planejamento") {
      return `${toPlainDate(days[0] ?? anchor, ZONE)} — ${toPlainDate(days[6] ?? anchor, ZONE)}`;
    }
    return formatLongDay(anchor);
  }, [mode, anchor, days]);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
      <header className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            aria-label="Período anterior"
            onClick={() => (mode === "mes" ? stepMonth(-1) : step(mode === "agenda" ? -1 : -7))}
          >
            <ChevronLeft className="size-4" aria-hidden />
          </Button>
          <Button variant="ghost" size="sm" onClick={goToday}>
            Hoje
          </Button>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Próximo período"
            onClick={() => (mode === "mes" ? stepMonth(1) : step(mode === "agenda" ? 1 : 7))}
          >
            <ChevronRight className="size-4" aria-hidden />
          </Button>
        </div>

        <h2 className="min-w-0 truncate text-[15px] font-semibold capitalize">{title}</h2>

        {/* Troca de modo: um `tablist` de verdade. Antes eram `role="tab"`
            soltos — sem `tabIndex` migratório, sem `aria-controls` e sem
            `tabpanel`, o teclado não conseguia andar entre os modos e o leitor de
            tela não sabia o que os botões controlavam. */}
        <div
          role="tablist"
          aria-label="Modo de visualização"
          className="ml-auto flex gap-1"
          onKeyDown={(event) => {
            const delta = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
            if (delta === 0) return;
            event.preventDefault();
            const current = MODES.findIndex((entry) => entry.id === mode);
            const next = (current + delta + MODES.length) % MODES.length;
            setMode(MODES[next]?.id ?? mode);
          }}
        >
          {MODES.map((item) => (
            <Button
              key={item.id}
              role="tab"
              id={`mode-tab-${item.id}`}
              aria-selected={mode === item.id}
              aria-controls={`mode-panel-${item.id}`}
              // Roving `tabIndex`: o `tablist` é uma parada só no teclado, e o
              // foco vai para a aba ativa — como em `App.tsx`.
              tabIndex={mode === item.id ? 0 : -1}
              size="sm"
              variant={mode === item.id ? "secondary" : "ghost"}
              onClick={() => setMode(item.id)}
            >
              {item.label}
            </Button>
          ))}
        </div>
      </header>

      <div className="flex min-h-0 flex-1 gap-3">
        <div
          id={`mode-panel-${mode}`}
          role="tabpanel"
          aria-labelledby={`mode-tab-${mode}`}
          className="flex min-h-0 min-w-0 flex-1 flex-col"
        >
          {mode === "semana" && (
            <WeekView days={days} byDay={byDay} now={now} scale={SCALE} onSelect={select} />
          )}
          {mode === "agenda" && (
            <AgendaView day={anchor} byDay={byDay} now={now} onSelect={select} onStep={step} />
          )}
          {mode === "mes" && (
            <MonthView
              anchor={anchor}
              byDay={byDay}
              now={now}
              onSelect={select}
              onStep={stepMonth}
            />
          )}
          {mode === "lista" && (
            <ListView
              items={items}
              responsibilities={responsibilities}
              now={now}
              onOpenList={onOpenList}
            />
          )}
          {mode === "planejamento" && (
            <PlanningView items={items} responsibilities={responsibilities} now={now} />
          )}

          {empty && <EmptyState onOpenList={onOpenList} />}
        </div>

        {selected !== null && <ItemPanel item={selected} onClose={clearSelection} />}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Semana
// ---------------------------------------------------------------------------

/**
 * Semana: 7 colunas por dia, e o eixo Y em pixels reais (D15/D16).
 *
 * Cada item é posicionado por `top`/`height` vindos de `placeInDay`. A versão
 * anterior desenhava o item na célula de **todas** as horas do seu dia, o que
 * multiplicava um agendamento de 9:00 por catorze cópias na tela.
 */
function WeekView({
  days,
  byDay,
  now,
  scale,
  onSelect,
}: {
  readonly days: readonly Instant[];
  readonly byDay: DayIndex<GridItem>;
  readonly now: Instant;
  readonly scale: GridScale;
  readonly onSelect: (item: GridItem) => void;
}) {
  const height = visibleHeight(scale);
  const today = toPlainDate(now, ZONE);
  const nowTop = timeToY(now, scale);
  const nowVisible = nowTop >= 0 && nowTop <= height;

  return (
    <div className="grid-scroll scrollbar-slim flex max-h-full min-h-0 flex-col overflow-auto rounded-[10px] border border-border/60">
      <div
        className="sticky top-0 z-20 grid grid-cols-[48px_repeat(7,minmax(96px,1fr))] border-b border-border/60"
        style={{ background: "var(--surface-raised)" }}
      >
        <div
          aria-hidden
          className="sticky left-0 z-10 border-r border-border/50"
          style={{ background: "var(--surface-raised)" }}
        />
        {days.map((day) => {
          const key = toPlainDate(day, ZONE);
          const isToday = key === today;
          return (
            <div
              key={key}
              className={`border-r border-border/50 px-2 py-1.5 text-center last:border-r-0 ${
                isToday ? "text-foreground" : "text-muted-foreground"
              }`}
            >
              <div className="text-[10px] uppercase tracking-[0.08em]">
                {formatWeekdayShort(day)}
              </div>
              <div
                className={`mx-auto mt-0.5 flex size-6 items-center justify-center rounded-full text-[12.5px] font-semibold tabular-nums ${
                  isToday ? "bg-primary text-primary-foreground" : ""
                }`}
              >
                {key.slice(8)}
              </div>
            </div>
          );
        })}
      </div>

      {/* O intrinsic-size acompanha a escala: se a densidade mudar, a reserva de
          espaço muda junto, senão a barra de rolagem dança a cada quadro. */}
      <div
        className="relative grid grid-cols-[48px_repeat(7,minmax(96px,1fr))]"
        style={{ height, "--day-column-intrinsic": `${height}px` } as CSSProperties}
      >
        {/* Eixo de horas. A coluna é `sticky` para continuar visível na rolagem,
            e `sticky left-0` também precisa do fundo opaco: um `sticky` sem
            fundo deixa o conteúdo passar por baixo. */}
        <div className="sticky left-0 z-10" style={{ background: "var(--surface-raised)" }}>
          {Array.from({ length: 24 - scale.dayStartHour }, (_, index) => {
            const hour = scale.dayStartHour + index;
            return (
              <div
                key={hour}
                style={{ height: scale.hourHeight }}
                className="flex items-start justify-end border-r border-b border-border/50 py-1 pr-2 text-right text-[10.5px] tabular-nums text-muted-foreground"
              >
                {hourLabels[hour] ?? `${String(hour).padStart(2, "0")}:00`}
              </div>
            );
          })}
        </div>

        {days.map((day) => (
          <DayColumn
            key={toPlainDate(day, ZONE)}
            day={day}
            items={itemsOn(byDay, toPlainDate(day, ZONE))}
            hourHeight={scale.hourHeight}
            height={height}
            scale={scale}
            onSelect={onSelect}
          />
        ))}

        {/* Linha do momento: um traço sobre a grade inteira, e não um item. */}
        {nowVisible && (
          <div
            aria-hidden
            className="pointer-events-none absolute inset-x-0 z-10 h-px bg-primary"
            style={{ top: nowTop }}
          >
            <div className="bg-primary absolute -left-1 -top-[3px] size-[7px] rounded-full" />
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Agenda
// ---------------------------------------------------------------------------

function AgendaView({
  day,
  byDay,
  now,
  onSelect,
  onStep,
}: {
  readonly day: Instant;
  readonly byDay: DayIndex<GridItem>;
  readonly now: Instant;
  readonly onSelect: (item: GridItem) => void;
  readonly onStep: (days: number) => void;
}) {
  const key = toPlainDate(day, ZONE);
  const items = [...itemsOn(byDay, key)].sort((a, b) => a.startsAt - b.startsAt);
  const isToday = key === toPlainDate(now, ZONE);

  return (
    <div className="flex max-h-full flex-col gap-2 overflow-auto">
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="sm" aria-label="Dia anterior" onClick={() => onStep(-1)}>
          <ChevronLeft className="size-4" aria-hidden />
        </Button>
        <span className={`text-[13px] ${isToday ? "font-semibold" : ""}`}>
          {formatWeekdayLong(day)}
        </span>
        <Button variant="ghost" size="sm" aria-label="Próximo dia" onClick={() => onStep(1)}>
          <ChevronRight className="size-4" aria-hidden />
        </Button>
      </div>

      {items.length === 0 ? (
        <p className="text-[13px] text-muted-foreground">Nada agendado para este dia.</p>
      ) : (
        <ol className="flex flex-col gap-1">
          {items.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => onSelect(item)}
                className="flex w-full items-center gap-3 rounded-[10px] border border-border/60 px-3 py-2 text-left transition-colors hover:bg-accent/40"
              >
                <span className="font-mono text-[12px] tabular-nums text-muted-foreground">
                  {formatClock(item.startsAt)}
                </span>
                <span className="min-w-0 flex-1">
                  <span
                    className={`block truncate text-[13px] ${layerClass(layerTone(item.layerId as LayerId))}`}
                  >
                    {item.title}
                  </span>
                </span>
                {item.isRecurring && (
                  <Repeat className="size-3.5 text-muted-foreground" aria-label="Recorrente" />
                )}
                <Badge variant="outline">{stateLabel(item.state)}</Badge>
              </button>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Mês
// ---------------------------------------------------------------------------

/**
 * Mês: grade de 6 linhas × 7 colunas.
 *
 * A grade começa na **segunda-feira da semana que contém o dia 1 do mês**, e
 * termina na semana que contém o último dia. A versão anterior começava seis dias
 * antes do âncora, o que fazia o dia 1 cair no meio da grade e o mês mostrar
 * parte do anterior e do seguinte sem critério.
 */
function MonthView({
  anchor,
  byDay,
  now,
  onSelect,
  onStep,
}: {
  readonly anchor: Instant;
  readonly byDay: DayIndex<GridItem>;
  readonly now: Instant;
  readonly onSelect: (item: GridItem) => void;
  readonly onStep: (months: number) => void;
}) {
  const anchorDate = toPlainDate(anchor, ZONE);
  const monthStart = plainDate(`${anchorDate.slice(0, 7)}-01`);
  // Quantos dias o mês tem: 28, 29, 30 ou 31. Contagem de calendário, não
  // horário — por isso vem do domínio e não de `new Date(ano, mês, 0)`, que
  // resolvia o dia zero no fuso da máquina.
  const lastDay = plainDate(
    `${anchorDate.slice(0, 7)}-${String(daysInMonth(monthStart)).padStart(2, "0")}`,
  );
  const gridStart = startOfWeek(fromPlainDate(monthStart, ZONE), ZONE);
  const today = toPlainDate(now, ZONE);

  return (
    <div className="flex max-h-full flex-col gap-2 overflow-auto">
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="sm" aria-label="Mês anterior" onClick={() => onStep(-1)}>
          <ChevronLeft className="size-4" aria-hidden />
        </Button>
        <span className="text-[13px] font-semibold capitalize">{formatMonthYear(anchor)}</span>
        <Button variant="ghost" size="sm" aria-label="Próximo mês" onClick={() => onStep(1)}>
          <ChevronRight className="size-4" aria-hidden />
        </Button>
      </div>

      <div className="grid grid-cols-7 gap-1">
        {WEEKDAY_LABELS.map((label) => (
          <div
            key={label}
            className="pb-1 text-center text-[10px] uppercase tracking-[0.06em] text-muted-foreground"
          >
            {label}
          </div>
        ))}

        {Array.from({ length: 42 }, (_, index) => {
          const day = addDaysInTz(gridStart, index, ZONE);
          const key = toPlainDate(day, ZONE);
          const items = itemsOn(byDay, key);
          const inMonth = key >= monthStart && key <= lastDay;
          const isToday = key === today;
          return (
            <div
              key={key}
              className={`min-h-[84px] rounded-[8px] border p-1.5 ${
                inMonth ? "border-border/60" : "border-transparent opacity-40"
              } ${isToday ? "ring-1 ring-primary/40" : ""}`}
            >
              <div
                className={`mb-1 flex size-5 items-center justify-center rounded-full text-[11px] tabular-nums ${
                  isToday
                    ? "bg-primary font-semibold text-primary-foreground"
                    : "text-muted-foreground"
                }`}
              >
                {key.slice(8)}
              </div>
              {items.slice(0, 3).map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onSelect(item)}
                  className={`block w-full truncate rounded-[4px] px-1 text-left text-[11px] ${layerClass(
                    layerTone(item.layerId as LayerId),
                  )}`}
                >
                  {item.title}
                </button>
              ))}
              {items.length > 3 && (
                <div className="px-1 text-[10.5px] text-muted-foreground">+{items.length - 3}</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Lista
// ---------------------------------------------------------------------------

function ListView({
  items,
  responsibilities,
  now,
  onOpenList,
}: {
  readonly items: readonly GridItem[];
  readonly responsibilities: readonly Responsibility[];
  readonly now: Instant;
  readonly onOpenList: () => void;
}) {
  // O atraso é derivado, nunca persistido (D11). Hoje e "próximos" saem do mesmo
  // predicado, com cortes diferentes.
  const pending = responsibilities.filter((r) => isOverdue(r, now) || r.state === "planejado");
  const overdue = pending.filter((r) => isOverdue(r, now));
  const upcoming = pending.filter((r) => !isOverdue(r, now));

  return (
    <div className="flex flex-col gap-4">
      <section>
        <h3 className="mb-2 text-[13px] font-semibold">Hoje</h3>
        {overdue.length === 0 ? (
          <p className="text-muted-foreground text-[13px]">Nada vencendo hoje.</p>
        ) : (
          overdue.map((r) => <ResponsibilityRow key={r.id} responsibility={r} late />)
        )}
      </section>
      <section>
        <h3 className="mb-2 text-[13px] font-semibold">Próximos dias</h3>
        {upcoming.length === 0 ? (
          <p className="text-muted-foreground text-[13px]">Sem prazos à frente.</p>
        ) : (
          upcoming.map((r) => <ResponsibilityRow key={r.id} responsibility={r} late={false} />)
        )}
      </section>
      <Button variant="ghost" size="sm" className="self-start" onClick={onOpenList}>
        <ListChecks className="size-3.5" />
        Abrir lista completa
      </Button>
    </div>
  );
}

function ResponsibilityRow({
  responsibility,
  late,
}: {
  readonly responsibility: Responsibility;
  readonly late: boolean;
}) {
  return (
    <div className="flex items-center gap-2 border-b border-border/50 py-1.5 last:border-0">
      <span className="min-w-0 flex-1 truncate text-[13px]">{responsibility.title}</span>
      {late ? <Badge variant="destructive">Atrasado</Badge> : null}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Planejamento
// ---------------------------------------------------------------------------

function PlanningView({
  items,
  responsibilities,
  now,
}: {
  readonly items: readonly GridItem[];
  readonly responsibilities: readonly Responsibility[];
  readonly now: Instant;
}) {
  // §6 responde duas perguntas separadas: quanto foi reservado e quanto saiu.
  const reserved = items.length;
  const open = responsibilities.filter((r) => isOpenState(r.state)).length;
  const late = responsibilities.filter((r) => isOverdue(r, now)).length;

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-3 gap-2">
        <Metric label="Reservado" value={reserved} />
        <Metric label="Em aberto" value={open} />
        <Metric label="Atrasado" value={late} />
      </div>
      <p className="text-muted-foreground text-[12px]">
        O planejamento mostra a diferença entre o tempo reservado e o executado. A comparação
        completa entra com o registro de execução da spec 03.
      </p>
    </div>
  );
}

function Metric({ label, value }: { readonly label: string; readonly value: number }) {
  return (
    <div className="rounded-[10px] border border-border/60 px-3 py-2">
      <div className="text-muted-foreground text-[11px]">{label}</div>
      <div className="font-mono text-[18px]">{value}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Painel do item — detalhe da seleção, não uma tela
// ---------------------------------------------------------------------------

function ItemPanel({ item, onClose }: { readonly item: GridItem; readonly onClose: () => void }) {
  return (
    <aside
      aria-label="Detalhe do item"
      className="flex w-[280px] shrink-0 flex-col gap-3 rounded-[10px] border border-border/60 p-3"
    >
      <div>
        <h3 className="text-[14px] font-semibold">{item.title}</h3>
        <p className="text-muted-foreground mt-0.5 text-[12px]">
          {toPlainDate(item.startsAt, ZONE)} · {formatClock(item.startsAt)}
        </p>
      </div>

      <div className="flex flex-wrap gap-1.5">
        <Badge variant="outline">
          {layerTone(item.layerId as LayerId) === "neutral" ? "Externo" : "Cecistudy"}
        </Badge>
        <Badge variant="secondary">{stateLabel(item.state)}</Badge>
        {item.isRecurring && <Badge variant="outline">Recorrente</Badge>}
        {item.overrideKind === "moved" && <Badge variant="outline">Remarcado</Badge>}
        {item.overrideKind === "cancelled" && <Badge variant="destructive">Cancelado</Badge>}
      </div>

      <div className="mt-auto flex gap-2">
        {/* As duas ações ficam desabilitadas até existirem: um botão que aceita
            clique e não faz nada é pior que um botão que declara que não está
            pronto. É a spec 02 que as liga (D17). */}
        <Button size="sm" variant="secondary" className="flex-1" disabled>
          Concluir
        </Button>
        <Button size="sm" variant="outline" className="flex-1" disabled>
          Reagendar
        </Button>
      </div>
      <Button size="sm" variant="ghost" onClick={onClose}>
        Fechar
      </Button>
    </aside>
  );
}

function EmptyState({ onOpenList }: { readonly onOpenList: () => void }) {
  return (
    <div className="flex flex-col items-center gap-2 py-10 text-center">
      <CalendarDays className="text-muted-foreground size-6" />
      <p className="text-[13px] font-medium">Nenhum evento neste período</p>
      <p className="text-muted-foreground max-w-sm text-[12px]">
        O Calendário mostra o que está agendado. Responsabilidades e blocos de estudo aparecem em
        Lista e Planejamento.
      </p>
      <Button size="sm" variant="outline" onClick={onOpenList}>
        <ListChecks className="size-3.5" />
        Abrir lista
      </Button>
    </div>
  );
}
