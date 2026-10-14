import type confetti from "canvas-confetti";
import type { Options } from "canvas-confetti";

/**
 * Celebração com confete no desktop (`SPEC-D-014` D92 / `SPEC-C-013` D6).
 *
 * Espelha `cecistudy/src/lib/celebrate.ts` — mesmos tipos e mesmas cores —, com
 * duas diferenças de escopo: o burst é mais contido (o desktop é denso) e o
 * disparo passa por um gate puro (`isCelebrationAllowed`) para ser testável sem
 * DOM. O núcleo testável (`CelebrationKind`, `CONFETTI_COLORS`,
 * `isCelebrationAllowed`) não importa `canvas-confetti`: a lib é carregada por
 * `import()` dinâmico só quando um marco dispara.
 */

/** Tipos de celebração. Mesmos nomes do mobile (paridade de vocabulário). */
export type CelebrationKind =
  | "tasks-done"
  | "reading-done"
  | "session-done"
  | "flashcards-done"
  | "sticker-unlocked"
  | "level-up"
  | "term-closed";

export const CELEBRATION_KINDS: readonly CelebrationKind[] = [
  "tasks-done",
  "reading-done",
  "session-done",
  "flashcards-done",
  "sticker-unlocked",
  "level-up",
  "term-closed",
];

/** Paleta de confetes alinhada ao design system (rose/green/blue/yellow). */
export const CONFETTI_COLORS = [
  "#E97891",
  "#D85F79",
  "#B94862",
  "#8BC7A2",
  "#43805B",
  "#609FB8",
  "#396D82",
  "#F9C74F",
] as const;

/**
 * Gate de acessibilidade, puro e testável. Confete é decoração; sob
 * `prefers-reduced-motion` ele não dispara (`SPEC-D-014` §7). A decisão é um
 * booleano, não o media query — assim o teste prova sem simular DOM.
 */
export function isCelebrationAllowed(reducedMotion: boolean): boolean {
  return !reducedMotion;
}

/** Respeita "reduzir movimento" do sistema (acessibilidade). */
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Resolve a instância do canvas-confetti só quando uma celebração dispara. */
let confettiPromise: Promise<typeof confetti> | null = null;
async function getConfetti(): Promise<typeof confetti> {
  if (!confettiPromise) {
    confettiPromise = import("canvas-confetti").then((m) => m.default);
  }
  return confettiPromise;
}

/** Burst único partindo do centro — base de todas as celebrações. */
async function burstFromCenter(count: number, opts: Options = {}) {
  const confetti = await getConfetti();
  confetti({
    colors: [...CONFETTI_COLORS],
    particleCount: count,
    spread: 72,
    startVelocity: 38,
    gravity: 0.9,
    ticks: 190,
    zIndex: 9999,
    origin: { x: 0.5, y: 0.62 },
    ...opts,
  });
}

/**
 * Explode confetes conforme o tipo de celebração. No-op sob `prefers-reduced-motion`.
 */
export function celebrate(kind: CelebrationKind): void {
  if (!isCelebrationAllowed(prefersReducedMotion())) return;

  switch (kind) {
    case "tasks-done":
      void burstFromCenter(120, { spread: 96, startVelocity: 44 });
      break;
    case "session-done":
    case "flashcards-done":
      void burstFromCenter(100, { spread: 80, startVelocity: 40 });
      break;
    case "sticker-unlocked":
      void burstFromCenter(140, { spread: 94, startVelocity: 40 });
      break;
    case "level-up":
      void burstFromCenter(150, { spread: 104, startVelocity: 46 });
      break;
    case "reading-done":
      void burstFromCenter(80, { spread: 70 });
      break;
    case "term-closed":
      // Virada de semestre é raro (uma vez por semestre): ganha o burst mais
      // generoso do desktop.
      void burstFromCenter(160, { spread: 104, startVelocity: 46 });
      break;
  }
}
