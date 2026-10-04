/**
 * Ponte do domínio do usuário.
 *
 * `SPEC-D-013` `D87`: identidade e preferência são substrate do shell, e o único
 * caminho de escrita é o módulo Configurações. Por isso esta pasta é de
 * `configuracoes` e não de `calendar` — `get_setting` e `set_setting` estão
 * registrados em `src-tauri/src/lib.rs` desde o começo do módulo Calendário e não
 * tinham consumidor TypeScript nenhum, o que o gate `npm run gate:contracts`
 * confirma e `src-tauri/src/lib.rs:37-39` negava.
 *
 * Esta camada fala o schema, não o SQL — a mesma regra que
 * `src/features/calendar/data/bridge.ts` cumpre.
 */

import { invoke } from "@/lib/ipc.ts";

/**
 * As chaves de configuração declaradas.
 *
 * Esta lista é a **mesma** que existe em três lugares, e ela precisa continuar
 * igual nos três:
 *
 * 1. aqui, como tipo;
 * 2. em `CHAVES_DE_CONFIGURACAO`, em `src-tauri/src/commands.rs`;
 * 3. no `CHECK` da coluna `chave`, em `src-tauri/migrations/0003_usuario.sql`.
 *
 * Uma chave fora daqui não chega ao Rust: o tipo é fechado de propósito. Com
 * `string`, o erro apareceria como recusa do backend em tempo de execução — o Rust
 * já recusa, com mensagem que lista as chaves, e isso é a última linha de defesa,
 * não a primeira.
 */
export const CHAVES_DE_CONFIGURACAO = [
  "regra_media.padrao",
  "fsrs.retencao_desejada",
  "google.integracao",
  "aparencia",
  "sincronizacao",
] as const;

/** Uma chave de configuração. `never` fora da lista, e não `string`. */
export type ChaveDeConfiguracao = (typeof CHAVES_DE_CONFIGURACAO)[number];

/** Aparência, de §5.4 linha 560. */
export type Aparencia = "light" | "dark" | "sistema";

/** Valor de cada chave. O formato é o que o Rust valida ao gravar. */
export type ValorDeConfiguracao = {
  "regra_media.padrao": unknown;
  "fsrs.retencao_desejada": number;
  "google.integracao": { ativa: boolean; calendario: string };
  aparencia: Aparencia;
  sincronizacao: unknown;
};

/**
 * Leitura de uma configuração.
 *
 * `ausente` é diferente de `valor: null`. Ausente é "a usuária nunca mexeu", e o
 * default é decisão do domínio, com regra escrita — não é o Rust adivinhando.
 */
export interface LeituraConfiguracao<T extends ChaveDeConfiguracao = ChaveDeConfiguracao> {
  readonly presente: boolean;
  readonly valor: ValorDeConfiguracao[T] | undefined;
}

/** Resposta do Rust para `get_setting`. */
interface RespostaSetting {
  readonly key: string;
  readonly value: unknown;
}

/**
 * Lê uma configuração.
 *
 * Configuração corrompida chega aqui como **erro**, não como `ausente`: o Rust
 * deixou de fazer `unwrap_or(Null)` (`SPEC-D-013` `D87`), e a consequência é que
 * a usuária vê o erro em vez de receber o default sem saber que o valor dela
 * tinha sumido.
 */
export async function lerConfiguracao<T extends ChaveDeConfiguracao>(
  chave: T,
): Promise<LeituraConfiguracao<T>> {
  const bruto = await invoke<RespostaSetting | null>("get_setting", { payload: { key: chave } });
  if (bruto === null || bruto === undefined) {
    return { presente: false, valor: undefined };
  }
  return { presente: true, valor: bruto.value as ValorDeConfiguracao[T] };
}

/**
 * Grava uma configuração.
 *
 * Único caminho de escrita de preferência no app — é o que `D87` exige, e o gate
 * `npm run gate:contracts` falha se outro módulo declarar `preferencia` em
 * `escreve`.
 */
export async function gravarConfiguracao<T extends ChaveDeConfiguracao>(
  chave: T,
  valor: ValorDeConfiguracao[T],
): Promise<void> {
  await invoke("set_setting", { payload: { key: chave, value: valor } });
}
