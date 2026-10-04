# Contrato de dados do grupo

Artefato único que os dois apps derivam. Decisão:
[`ADR-007`](../../../docs/decisoes/ADR-007-contrato-de-dados-unico-e-bases-separadas.md).
Dono: o app desktop, `cecistudy-desktop`. Ver também a
[`ADR-009`](../../../docs/decisoes/ADR-009-tradutor-e-base-fisica-por-app.md).

> **Por que mora aqui e não no mobile.** O `ADR-008` removeu `cecistudy-rust/`,
> que era onde este contrato vivia. O mobile **não** é o dono: ele consome. Os
> testes dele escrevem e leem estes arquivos por caminho relativo
> (`../../../cecistudy-desktop/contratos/dados/`), e o CI do mobile faz um
> segundo checkout deste repositório para que isso funcione.

## O que mora aqui

| Caminho                | O que fixa                                                                                                                                |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `schema.sql`           | tabelas, colunas, tipos, chaves, nulabilidade, versão                                                                                     |
| `backup-v2-spec.md`    | envelope `cecistudy-user-backup`: o que entra, em que ordem, formato de migração                                                          |
| `canonical-json-v1.md` | a serialização de comparação: chaves alfabéticas, strings NFC, números ECMAScript `Number::toString` (sem `.0`), arrays na ordem original |
| `verify-schema.mjs`    | verificador: prova que um `schema.sql` concreto está em conformidade                                                                      |
| `golden/`              | os fixtures byte-a-byte, por coleção                                                                                                      |

### `golden/`

Todos os arquivos são **canonical JSON v1**.

| Arquivo                               | Conteúdo                                                                           |
| ------------------------------------- | ---------------------------------------------------------------------------------- |
| `full_backup.empty.json`              | envelope BackupV2 do estado zerado (exportedAt fixo)                               |
| `full_backup.sample.json`             | envelope BackupV2 do estado cênico (`src/data/fixtures/goldenSample.ts` no mobile) |
| `collections/empty/<coleção>.json`    | payload de cada coleção do estado vazio                                            |
| `collections/sample/<coleção>.json`   | payload de cada coleção do estado cênico                                           |
| `migrations/legacy_payload.v<n>.json` | payload legado por versão, para o gate de migração                                 |
| `canonical_hash_vectors.json`         | vetores de hash canônico                                                           |

## Regenerar

Só o mobile gera, porque só ele tem o estado cênico:

```bash
# no repositório do mobile
GOLDEN_WRITE=1    npm run test -- src/lib/__tests__/goldenFixtures.test.ts
MIGRATION_WRITE=1 npm run test -- src/lib/__tests__/migrationFixtures.test.ts
```

Depois **revise o diff**. Nunca editar um golden na mão — o diff tem que vir só
da fonte.

## Débito aberto: o gate de paridade Rust não existe

> ⚠️ **Isto aqui é contrato sem consumidor no lado do desktop.**

Os testes que consumiam estes goldens byte-a-byte
(`golden_parity_test.rs` em `cecistudy-common` e `migration_parity_test.rs` em
`cecistudy-data`) viviam em `cecistudy-rust/crates/`, que o `ADR-008` removeu.
Hoje `src-tauri/` **não lê nenhum golden** — verificado por busca.

Consequência: enquanto o gate não existir, os goldens são _write-and-read_ só do
lado do TypeScript. Isso é mais fraco do que a ADR-007 pretende, e é a pendência
de reconstruir o gate de paridade em `src-tauri/`.

## Contrato relacionado

- Versões: `SCHEMA_VERSION` em `packages/data/src/schema.ts` (mobile) e
  `USER_SCHEMA_VERSION` (ver `ADR-007` — hoje ainda há dois valores para a mesma
  constante, débito C2).
- Chaves do payload = `buildBackupData` de `packages/data/src/persistentData.ts`
  (banco persistido **menos** bancos estáticos `approaches`/`questions`).
- Lista fechada de campos que a camada clínica pode atravessar:
  `SPEC-M-013` `D2` (mobile) e `SPEC-C-013` `D5` (tradutor no desktop).
