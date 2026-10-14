/**
 * Arquivo único de voz do desktop (`SPEC-D-014` D90 / `SPEC-C-013` D7).
 *
 * Todo texto de identidade do produto mora aqui; componente nenhum escreve
 * texto de identidade inline (I5). Escopo reduzido de propósito (D4): a voz é
 * comum aos dois apps — pt-BR, acolhedora, direta, em minúsculas, com o coração
 * na marca — mas **sem** metáforas "cozy" ("cantinho"), sem diminutivos e sem
 * emoji decorativo, porque o desktop é denso e silencioso (`DESIGN.md`).
 *
 * Dado puro, sem imports: é auditável e testável (`copy.test.ts`) — inclusive a
 * regra negativa de D4.
 */

/** Identidade do produto. Uma marca visível, uma técnica, um personagem. */
export const IDENTIDADE = {
  /** Marca visível, com o coração (`ADR-011`). */
  marca: "cecistudy ♡",
  /** Nome técnico, sem o glyph: `package.json`, crate, `identifier`. */
  marcaTecnica: "cecistudy",
  /** A assistente e o mascote são o mesmo personagem (`ADR-011`). */
  assistente: "cecinho",
  /** Rótulo padrão da dica do personagem. */
  rotuloDica: "dica do cecinho",
} as const;

/** Mesagens de estado vazio, acolhedoras e curtas. */
export const VAZIO = {
  /** Busca sem resultado. Recebe o termo consultado. */
  nadaEncontrado: (termo: string) => `nada encontrado para "${termo}".`,
  /** Módulo declarado que ainda não tem tela. */
  moduloEmConstrucao: "esta tela ainda não existe — a entrada é decisão de produto.",
  /** Workspace sem tela. */
  workspaceEmConstrucao: "este workspace ainda não tem tela.",
} as const;

/** Confirmações gentis. Sem alarmismo, sem exclamação. */
export const FEEDBACK = {
  guardado: "guardado ♡",
  sincronizado: "sincronizado ♡",
} as const;
