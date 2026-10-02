/**
 * Item posicionado da Semana (F0 · DP6).
 *
 * Extraído do corpo de `WeekView` por dois motivos que não são aesthetics:
 *
 * 1. **Fronteira de render.** Um item é a unidade que muda. Clicar num item não
 *    muda o conjunto — muda a seleção — e seleção não mora na grade (DP6). Com o
 *    botão inline, cada clique re-executava o `placeInDay` e o `formatClock` de
 *    **todos** os itens da coluna, porque o `map` inteiro é re-criado. Com o
 *    componente memoizado, os outros itens não renderizam.
 * 2. **Fronteira do arraste.** A Fase F2 escreve `style.top`/`style.height` a
 *    60 Hz durante o gesto. O alvo dessa escrita precisa ser um nó pequeno e
 *    isolado; se o estado do arraste morar na tela, cada `pointermove` re-renderiza
 *    a semana inteira (ver DP6 e A4 de `00-fundacao.md`).
 *
 * Tudo que este componente recebe é **estável**: `placement` é número, `item` é o
 * objeto que veio da leitura, e `onSelect` é um `useCallback`. Se algum deles
 * passar a ser recriado por render, o `memo` deixa de acertar — e aí o defeito é
 * do chamador, não daqui.
 */

import { memo } from "react";

import type { GridItem } from "../data/bridge.ts";
import type { Placement } from "../domain/grid-scale.ts";
import { layerClass, layerTone } from "../domain/layers.ts";
import type { LayerId } from "../domain/types.ts";
import { formatClock } from "./format.ts";

export interface TimedItemProps {
  readonly item: GridItem;
  readonly placement: Placement;
  readonly column: number;
  readonly columns: number;
  readonly onSelect: (item: GridItem) => void;
}

export const TimedItem = memo(function TimedItem({
  item,
  placement,
  column,
  columns,
  onSelect,
}: TimedItemProps) {
  const width = 100 / columns;

  return (
    <button
      type="button"
      onClick={() => onSelect(item)}
      title={`${item.title} — ${formatClock(item.startsAt)}`}
      className={`absolute overflow-hidden rounded-[5px] border px-1 py-0.5 text-left text-[10.5px] leading-tight ${layerClass(
        layerTone(item.layerId as LayerId),
      )} ${item.overrideKind === "cancelled" ? "opacity-50 line-through" : ""}`}
      style={{
        top: placement.top,
        height: placement.height,
        left: `calc(${column * width}% + 2px)`,
        width: `calc(${width}% - 4px)`,
      }}
    >
      <span className="block truncate font-medium">{item.title}</span>
      {placement.height > 30 && (
        <span className="block truncate text-[10px] opacity-80">{formatClock(item.startsAt)}</span>
      )}
    </button>
  );
});
