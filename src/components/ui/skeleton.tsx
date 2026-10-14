import { cn } from "@/lib/utils";

/**
 * Esqueleto de carregamento.
 *
 * Três decisões, e as três são de motion:
 *
 * 1. **Sem `animate-pulse`.** O pulso é `infinite`, e o bloco de
 *    `prefers-reduced-motion: reduce` corta a *duração* para 0.01ms — o que
 *    transforma um pulso lento num **strobe** de 100 Hz (WCAG 2.3.1, nível A). A
 *    varredura fica dentro de `no-preference`, então quem pediu menos movimento
 *    recebe o preenchimento estático sem precisar de regra de reset.
 * 2. **`bg-muted`, não `bg-primary/10`.** Um esqueleto tingido de cor primária
 *    compete com o conteúdo que ele vai substituir, e em carga rápida o esqueleto
 *    pisca na tela por menos tempo do que leva para ser lido.
 * 3. **`role="status"`** com texto acessível. Um `div` vazio é lido como nada; o
 *    esqueleto precisa anunciar que está carregando.
 */
function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      role="status"
      aria-live="polite"
      aria-busy="true"
      className={cn(
        "skeleton-shimmer relative overflow-hidden rounded-md bg-muted motion-reduce:animate-none",
        className,
      )}
      {...props}
    >
      <span className="sr-only">Carregando</span>
    </div>
  );
}

export { Skeleton };
