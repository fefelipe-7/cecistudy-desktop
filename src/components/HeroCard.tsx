/**
 * Hero do desktop (`SPEC-D-014` D92): a abertura de uma tela de destaque.
 *
 * Espelha `cecistudy/src/components/ui/HeroCard.tsx` (molde de apresentação:
 * dado chega por props, nada é calculado aqui) com os tokens do desktop — e o
 * acento de marca (`--brand`, D91) no lugar do tema cozy do mobile. Hero só em
 * tela de destaque (D6): usá-lo em toda tela é ruído.
 */
import { Sparkles } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import { Mascote, type MascoteExpression } from "./Mascote";

export interface HeroCardProps {
  /** eyebrow em minúsculas: "workspace acadêmico". */
  eyebrow: string;
  /** Título da tela — o herói é o conteúdo, não a marca. */
  title: string;
  /** Frase de resumo, uma linha. */
  summary?: ReactNode;
  /** Ação primária, quando a tela tem uma. */
  action?: { label: string; onClick: () => void };
  expression?: MascoteExpression;
  accent?: "brand" | "neutral";
  testId?: string;
}

const ACCENTS = {
  brand: "from-card to-secondary/60 border-border",
  neutral: "from-card to-muted/50 border-border",
} as const;

export const HeroCard: React.FC<HeroCardProps> = ({
  eyebrow,
  title,
  summary,
  action,
  expression = "welcome-wave",
  accent = "brand",
  testId = "hero-card",
}) => (
  <section
    data-testid={testId}
    className={cn(
      "relative overflow-hidden rounded-2xl bg-gradient-to-br p-5 shadow-float",
      ACCENTS[accent],
    )}
  >
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <p className="flex items-center gap-1.5 text-[11px] font-medium lowercase tracking-wide text-brand">
          <Sparkles className="size-3" />
          {eyebrow}
        </p>
        <h1 className="mt-1 break-words text-[22px] font-semibold tracking-[-0.01em] text-foreground">
          {title}
        </h1>
      </div>
      {action && (
        <Button variant="outline" size="sm" className="shrink-0" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
    {summary && (
      <p className="mt-2.5 max-w-[92%] text-[12.5px] leading-relaxed text-muted-foreground">
        {summary}
      </p>
    )}
    <Mascote
      expression={expression}
      className="pointer-events-none absolute -bottom-2 -right-2 size-16 opacity-95"
      decorative
    />
  </section>
);
