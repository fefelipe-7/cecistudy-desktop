/**
 * "Dica do cecinho" no desktop (`SPEC-D-014` D92): mascote + uma frase, no
 * empty state. Espelha `cecistudy/src/components/views/home/CecinhoTip.tsx`
 * com a voz reduzida de D4 — o rótulo vem de `lib/copy.ts` (nunca inline) e o
 * ícone é o `Sparkles` do lucide, não emoji decorativo.
 */
import { Sparkles } from "lucide-react";

import { IDENTIDADE } from "@/lib/copy";

import { Mascote } from "./Mascote";

interface CecinhoTipProps {
  tip: string;
}

export const CecinhoTip: React.FC<CecinhoTipProps> = ({ tip }) => (
  <div className="flex items-center gap-3 rounded-xl border border-border bg-card p-4 text-left shadow-float">
    <Mascote expression="celebrate-small" className="size-12 shrink-0" decorative />
    <div className="min-w-0">
      <div className="flex items-center gap-1.5 text-[11px] font-semibold text-foreground">
        <Sparkles className="size-3.5 text-brand" />
        <span>{IDENTIDADE.rotuloDica}</span>
      </div>
      <p className="mt-1 text-[12px] leading-relaxed text-muted-foreground">{tip}</p>
    </div>
  </div>
);
