/**
 * Vocabulário de expressões do cecinho — nomes canônicos, sem assets.
 *
 * `SPEC-C-013` §5.1 / `SPEC-D-014` D3: os 24 nomes são o contrato de identidade
 * e são iguais nos dois apps. Este módulo é **dado puro** (nenhum `.webp`), para
 * que o teste de paridade (I2) rode sob `node --test` sem bundler; o mapa
 * nome → arquivo e o componente vivem em `Mascote.tsx`.
 *
 * A lista é a fonte do tipo `MascoteExpression`; o `Record` do componente é
 * tipado por ela, então `tsc` recusa um mapa com chave a mais ou a menos — o
 * teste garante o conteúdo, o typecheck garante a forma.
 */

export const MASCOTE_EXPRESSION_NAMES = [
  "welcome-wave",
  "listening-hello",
  "focus-ready",
  "reading-curious",
  "library-shelf",
  "writing-note",
  "review-card",
  "quiz-ready",
  "correct-soft",
  "try-again",
  "empty-invite",
  "done-calm",
  "celebrate-small",
  "class-ready",
  "field-prepare",
  "supervision-reflect",
  "connection-link",
  "research-tcc",
  "writing-flow",
  "boundaries-care",
  "sync-wait",
  "loading-patient",
  "no-results",
  "pause-kind",
] as const;

export type MascoteExpression = (typeof MASCOTE_EXPRESSION_NAMES)[number];
