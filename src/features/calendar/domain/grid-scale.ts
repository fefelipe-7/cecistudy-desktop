/**
 * Geometria da grade (spec 02, D15).
 *
 * A regra que este arquivo existe para tornar verdadeira: existe **uma** escala.
 * Arraste, redimensionamento, criação e leitura de item usam `timeToY`/`yToTime`,
 * de modo que o que se vê é exatamente o que se grava. Se a posição do pixel fosse
 * calculada em dois lugares, uma hora de verão ou um `dayStartHour` diferente
 * fariam o item ser gravado no horário que ele não ocupa.
 *
 * Puro: sem React, sem `invoke`, sem `Date.now()`. O fuso entra por parâmetro e o
 * "agora" também, porque a linha do momento é dado, não leitura de relógio.
 */

import {
  addDaysInTz,
  fromWallClock,
  minutes,
  toPlainDate,
  toWallClock,
  type IanaTimeZone,
  type Instant,
  type Minutes,
} from "./time.ts";

/** Densidade da grade. 56px é o default do desktop (D16). */
export interface GridScale {
  /** Primeira hora visível. O eixo Y não precisa começar à meia-noite. */
  readonly dayStartHour: number;
  readonly hourHeight: number;
  /** Granularidade do encaixe. O default é 15 minutos (D15). */
  readonly snapMinutes: Minutes;
  readonly timeZone: IanaTimeZone;
}

export const DEFAULT_SCALE: Omit<GridScale, "timeZone"> = {
  dayStartHour: 6,
  hourHeight: 56,
  snapMinutes: minutes(15),
};

/** Altura visível do eixo, em pixels: da primeira hora até o fim do dia. */
export function visibleHeight(scale: GridScale): number {
  return (24 - scale.dayStartHour) * scale.hourHeight;
}

/**
 * Pixels do topo da grade para um instante.
 *
 * O cálculo passa pelo **relógio de parede** do fuso, e não por hora UTC: às 09:00
 * de Brasília o deslocamento para UTC muda, e dividir por 3.600.000 colocaria o
 * item duas horas fora do lugar em parte dos dias do ano.
 */
export function timeToY(value: Instant, scale: GridScale): number {
  return yInDay(value, value, scale);
}

/**
 * O mesmo cálculo, mas ancorado no dia `day` em vez do dia do próprio instante.
 *
 * A diferença importa na meia-noite: `00:00` do dia seguinte tem hora-de-parede
 * zero, e medir pelo dia do próprio instante o colocaria `dayStartHour` acima do
 * topo, quando na verdade ele é o **fim** da coluna anterior.
 */
function yInDay(value: Instant, day: Instant, scale: GridScale): number {
  const wall = toWallClock(value, scale.timeZone);
  const clock = new Date(wall);
  const sameDay = toPlainDate(value, scale.timeZone) === toPlainDate(day, scale.timeZone);
  const hours = sameDay
    ? clock.getUTCHours() + clock.getUTCMinutes() / 60 + clock.getUTCSeconds() / 3600
    : // Depois da meia-noite do dia pedido: a posição é a última hora do dia.
      24;
  return (hours - scale.dayStartHour) * scale.hourHeight;
}

/**
 * Altura útil do eixo, em pixels: nunca negativa e nunca acima da faixa visível.
 *
 * `yToTime` recusa aceitar um `y` fora da grade porque um ponteiro arrastado para
 * fora da janela não pode gravar um instante no dia seguinte — ele tem de ser
 * recortado pela grade, que é quem sabe onde termina o dia. O recorte fica
 * separado em `clampY` para que o chamador possa distinguir "o ponteiro está
 * dentro" de "o ponteiro foi recortado".
 */
export function clampY(y: number, scale: GridScale): number {
  return Math.min(Math.max(y, 0), visibleHeight(scale));
}

/** O inverso de `timeToY`: pixels do topo para o instante que o pixel representa. */
export function yToTime(y: number, day: Instant, scale: GridScale): Instant {
  const clamped = clampY(y, scale);
  const hours = clamped / scale.hourHeight;
  const wholeHours = Math.floor(hours);
  const minutes = Math.round((hours - wholeHours) * 60);
  // A meia-noite do dia pedido vira o ponto de partida do relógio de parede, de
  // modo que `y = 0` significa `dayStartHour:00` **naquele dia**, e não no dia 0.
  const dayWall = toWallClock(addDaysInTz(day, 0, scale.timeZone), scale.timeZone);
  const base = dayWall + (scale.dayStartHour + wholeHours) * 3_600_000 + minutes * 60_000;
  return fromWallClock(base, scale.timeZone);
}

/** Encaixa um instante na grade. É o que o arrasto e a criação usam ao persistir. */
export function snapTime(value: Instant, scale: GridScale): Instant {
  const step = scale.snapMinutes * 60_000;
  return (Math.round(value / step) * step) as Instant;
}

/**
 * Encaixa um pixel antes de converter, para o ponteiro não sair do passo.
 *
 * O passo sai da **escala**, e não de um literal: `snapMinutes` é uma duração e
 * o pixel depende de `hourHeight`. A versão anterior assumia 4 pixels por minuto,
 * o que só bateria com `snapMinutes: 15` **e** `hourHeight: 60` — com o 56px de
 * D16 o encaixe ficava 4,3× fora do passo, e o item era gravado num horário que
 * não era o que a usuária via.
 *
 * `snapY` é o **único** caminho de encaixe em pixel: o arraste enquadra aqui e
 * só então chama `yToTime`, para que o que se vê gravado seja o que se vê.
 */
export function snapY(y: number, scale: GridScale): number {
  const step = (scale.snapMinutes / 60) * scale.hourHeight;
  // Passo zero (ou `hourHeight` zero) não tem resposta: devolveria `NaN`, e um
  // `NaN` no `top` de um item o some da tela sem aviso nenhum. Sem passo não há
  // encaixe, e o pixel cru é a resposta honesta.
  if (!Number.isFinite(step) || step <= 0) return y;
  return Math.round(y / step) * step;
}

/** Um pedaço de item desenhável, já em pixels. */
export interface Placement {
  readonly top: number;
  readonly height: number;
  /** `true` quando o pedaço é a continuação de um item que começou no dia anterior. */
  readonly continued: boolean;
  /** `true` quando o item segue para o dia seguinte. */
  readonly continues: boolean;
}

const MIN_VISIBLE_PX = 18;

/** `Math.min`/`Math.max` devolvem `number` e apagam a marca; isto a reconstitui. */
function clamp(value: Instant, min: Instant, max: Instant): Instant {
  return Math.min(Math.max(value, min), max) as Instant;
}

/**
 * Posiciona um item dentro de **um** dia da grade.
 *
 * O dia é passado separadamente porque um item que atravessa a meia-noite pertence
 * a duas colunas: desenhá-lo inteiro na coluna do início esconderia a parte da
 * manhã do dia seguinte. Por isso isto devolve um array — um item vira um pedaço por
 * dia atravessado, e a coluna decide o que fazer com cada um.
 */
export function placeInDay(
  startsAt: Instant,
  endsAt: Instant,
  day: Instant,
  scale: GridScale,
): readonly Placement[] {
  const dayStart = addDaysInTz(day, 0, scale.timeZone);
  const dayEnd = addDaysInTz(day, 1, scale.timeZone);
  const from = clamp(startsAt, dayStart, dayEnd);
  const to = clamp(endsAt, dayStart, dayEnd);
  if (to <= from) return [];

  const ceiling = visibleHeight(scale);
  const top = Math.max(0, yInDay(from, day, scale));
  const bottom = Math.min(ceiling, yInDay(to, day, scale));
  // Fora da faixa visível não há o que desenhar: um item que termina às 02:00
  // não pode ganhar um retângulo fantasma às 06:00 só por causa do mínimo.
  if (bottom <= 0 || top >= ceiling) return [];

  const height = Math.max(bottom - top, MIN_VISIBLE_PX);

  return [
    {
      top,
      height,
      // "continuation" é uma propriedade do item, não do recorte: o pedaço sabe
      // que continua alguma coisa vinda de antes, mesmo que comece no topo.
      continued: startsAt < dayStart,
      continues: endsAt > dayEnd,
    },
  ];
}

/**
 * Distribui itens que se sobrepõem em faixas, para que nenhum fique escondido
 * atrás do outro. Sobreposição é permitida e derivada (D7) — isto é apenas
 * legibilidade, nunca bloqueio.
 *
 * Devolve a **coluna** de cada item e o total de colunas do grupo, que é o que
 * permite ao CSS dividir a largura sem a usuária precisar passar o mouse por cima
 * para descobrir o que existe embaixo.
 */
export interface ColumnedItem<T> {
  readonly item: T;
  readonly column: number;
  readonly columns: number;
}

/**
 * Distribui itens que se sobrepõem em faixas, para que nenhum fique escondido
 * atrás do outro. Sobreposição é permitida e derivada (D7) — isto é apenas
 * legibilidade, nunca bloqueio.
 *
 * `span` diz onde cada item começa e termina; a lista precisa vir **já ordenada
 * por início**, que é o que a leitura da janela devolve.
 */
export function columnize<T>(
  items: readonly T[],
  span: (item: T) => { readonly from: number; readonly to: number },
): readonly ColumnedItem<T>[] {
  const out: ColumnedItem<T>[] = [];
  // Um "cluster" é um grupo de itens encadeados por sobreposição. Itens de clusters
  // diferentes nunca disputam largura, mesmo que a lista inteira seja longa.
  let cluster: T[] = [];
  let clusterEnd = Number.NEGATIVE_INFINITY;

  const flush = (): void => {
    if (cluster.length === 0) return;
    // Uma faixa por vez: um item entra na primeira faixa cujo último item já
    // terminou, senão abre-se uma nova. Isso é o mínimo de colunas possível.
    const lanes: Array<{ end: number; readonly members: T[] }> = [];
    for (const item of cluster) {
      const { from, to } = span(item);
      const lane = lanes.find((candidate) => candidate.end <= from);
      if (lane) {
        lane.members.push(item);
        lane.end = Math.max(lane.end, to);
      } else {
        lanes.push({ end: to, members: [item] });
      }
    }
    lanes.forEach((lane, column) => {
      for (const item of lane.members) {
        out.push({ item, column, columns: lanes.length });
      }
    });
    cluster = [];
    clusterEnd = Number.NEGATIVE_INFINITY;
  };

  for (const item of items) {
    const { to } = span(item);
    if (cluster.length > 0 && to > clusterEnd) {
      // Este item ainda está em curso quando o cluster termina: ele pertence a um
      // cluster maior, então o cluster anterior é fechado primeiro.
      flush();
    }
    cluster.push(item);
    clusterEnd = Math.max(clusterEnd, to);
  }
  flush();
  return out;
}
