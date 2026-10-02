/**
 * Coluna de um dia na Semana (F0 · P5c · DP6).
 *
 * Uma coluna é o **custo real** da grade: ela recebe os itens daquele dia, ordena,
 * distribui em faixas e posiciona. Três coisas moravam aqui e nenhuma delas é o
 * trabalho que justifica re-renderizar quando a usuária seleciona outro item.
 *
 * - **As 18 células de hora viraram textura.** Cada coluna desenhava 18 `div`s
 *   vazias só para ter uma borda a cada hora — 126 nós no total, com
 *   recalculo de estilo em cada um por quadro de rolagem. A mesma linha agora é
 *   um `repeating-linear-gradient` no fundo da coluna. O **eixo de horas**
 *   (rótulos) continua com elementos, porque precisam de texto.
 * - **`ordered`/`lanes` não são mais recalculados** a cada render: eles moram no
 *   corpo do componente memoizado, então só são recalculados quando a lista
 *   daquele dia muda de fato.
 * - **`placeInDay` sai do `map`** para dentro do `TimedItem` memoizado, para que
 *   o custo de `Intl` por item seja pago uma vez por item, e não uma vez por
 *   render de coluna.
 */

import { memo } from "react";

import type { GridItem } from "../data/bridge.ts";
import { columnize, placeInDay, type GridScale } from "../domain/grid-scale.ts";
import type { Instant } from "../domain/time.ts";
import { TimedItem } from "./TimedItem.tsx";

export interface DayColumnProps {
  /** O dia civil desta coluna, usado como `key` pelo chamador. */
  readonly day: Instant;
  readonly items: readonly GridItem[];
  readonly hourHeight: number;
  readonly height: number;
  readonly scale: GridScale;
  readonly onSelect: (item: GridItem) => void;
}

export const DayColumn = memo(function DayColumn({
  day,
  items,
  hourHeight,
  height,
  scale,
  onSelect,
}: DayColumnProps) {
  // Ordenar por início é requisito do `columnize`: sem isso as faixas seriam
  // montadas na ordem de chegada e o resultado seria instável entre renders.
  const ordered = [...items].sort((a, b) => a.startsAt - b.startsAt);
  const lanes = columnize(ordered, (item) => ({ from: item.startsAt, to: item.endsAt }));

  // A textura substitui `Array.from({ length: 24 - dayStartHour })` de células.
  // `repeating-linear-gradient` mede em % da altura do elemento, então acompanha
  // `hourHeight` sem recalcular nada quando a densidade muda.
  const texture = `repeating-linear-gradient(to bottom, var(--color-border) 0 1px, transparent 1px ${hourHeight}px)`;

  return (
    <div
      className="day-column relative border-r border-border/50 last:border-r-0"
      style={{ height, backgroundImage: texture }}
    >
      {lanes.map(({ item, column, columns }) => {
        const [placement] = placeInDay(item.startsAt, item.endsAt, day, scale);
        if (!placement) return null;
        return (
          <TimedItem
            key={item.id}
            item={item}
            placement={placement}
            column={column}
            columns={columns}
            onSelect={onSelect}
          />
        );
      })}
    </div>
  );
});
