# Identidade — cecinho, voz e marca no desktop

> Fonte de verdade no app para a
> [`SPEC-D-014`](../../../docs/specs/desktop/SPEC-D-014-identidade-no-desktop.md).
> Espelha a comum [`SPEC-C-013`](../../../docs/specs/comum/SPEC-C-013-identidade-do-produto-mascote-voz-e-marca.md)
> (D1..D7). Decisões do app: `D89`..`D94`.
> Estado: implementado em 2026-10-10 (D89..D94).

A identidade é **apresentação**: nenhum destes itens é regra de domínio, nada
toca `src-tauri/` além do rebrand técnico de D93 (`ADR-006`).

## D89 — Componente `Mascote`

- Os 24 nomes canônicos de `SPEC-C-013` §5.1 vivem em
  `src/components/mascote-expressions.ts` — dado puro, sem assets, para o teste
  de paridade rodar sob `node --test` (JSX não passa pelo
  `--experimental-strip-types`, e o teste precisa importar o contrato sem
  bundler).
- O componente é `src/components/Mascote.tsx`: mapa `Record<MascoteExpression,
  string>` (o tipo obriga cobertura exata dos 24 nomes) e `<img>` com
  `decorative`/`alt` para acessibilidade.
- Assets: `src/assets/mascote/*.webp` (24, binários do mobile preservados).
- Teste: `src/components/mascote.test.ts` (I2: as 24 canônicas, na ordem, únicas).

## D90 — Arquivo único de voz

- `src/lib/copy.ts`: `IDENTIDADE` (marca `cecistudy ♡`, técnica `cecistudy`,
  assistente `cecinho`, rótulo da dica), `VAZIO` e `FEEDBACK`. Componente nenhum
  escreve texto de identidade inline (`src/App.tsx` consome `VAZIO.nadaEncontrado`).
- `src/lib/tips.ts`: `pickTip(ctx, now)` determinístico, prioridade do mais
  específico, relógio injetado.
- Testes: `src/lib/copy.test.ts` (I1 e a regra negativa de D4 — sem "cantinho",
  sem emoji decorativo) e `src/lib/tips.test.ts` (determinismo e prioridade).

## D91 — Token de marca em `src/styles.css`

- `--brand: oklch(0.64 0.147 7.5)` em `:root` — derivado de `#D4617C` (o
  rosa-marca do mobile, único valor de cor comum aos apps, I4); `.dark` usa
  `oklch(0.72 0.15 7.5)` para contraste AA.
- Registrado em `@theme inline` como `--color-brand`/`--color-brand-foreground`
  (`bg-brand`, `text-brand`). Separado de `--primary` de propósito: `--primary`
  é a cor de **ação** do sistema, `--brand` é a identidade.

## D92 — Hero, `CecinhoTip` e celebração

- `src/components/HeroCard.tsx`: hero de tela de destaque. Hoje, a Home
  (`src/App.tsx`, `ModuloEmConstrucao` com `destaque`) — a tela que o app abre.
- `src/components/CecinhoTip.tsx`: mascote + rótulo de `copy.ts` + dica de
  `tips.ts`; aparece nos empty states (`ModuloEmConstrucao`,
  `WorkspaceEmConstrucao`).
- `src/lib/celebrate.ts`: espelho de `cecistudy/src/lib/celebrate.ts` (mesmos 7
  tipos, mesmas cores), com gate puro `isCelebrationAllowed` — sob
  `prefers-reduced-motion` o confete não dispara. Dependência `canvas-confetti`
  é carregada por `import()` dinâmico, só no disparo.
- Teste: `src/lib/celebrate.test.ts`.

## D93 — Rebrand técnico completo

- `package.json`: `name` = `cecistudy-desktop`.
- `src-tauri/Cargo.toml`: crate `cecistudy`, lib `cecistudy_lib`;
  `src-tauri/src/main.rs` segue a lib nova.
- `src-tauri/tauri.conf.json`: `productName` = `cecistudy`, `identifier` =
  `com.cecistudy.desktop`, título da janela = `cecistudy ♡`.
- `index.html:7,9`: título e `meta author`.
- Comentários de `src-tauri/migrations/*.sql`.
- **O arquivo do banco não mudou** (`campus.sqlite`, `src-tauri/src/db.rs:29`):
  renomear base existente é perda de dado, e D93 não o inclui. Pendência de
  produto se a dona quiser o nome novo na base.

## D94 — Utilitário de IA

- `src/shell/navigation.ts`: `UTILITARIOS` usa `cecinho`; `src/App.tsx` segue
  (`ICONE_UTILITARIO`, `ROTULO_UTILITARIO`). O rótulo antigo não volta:
  `navigation.test.ts` trava a lista.

## Verificação

`npm run verify` (typecheck, lint, 268 testes, 6 golden, fronteiras, 132 testes
Rust, clippy, fmt, check) — medido verde em 2026-10-10. `npm run
tauri:build:exe` gera `cecistudy.exe` (`src-tauri/target/release/`). Critérios
§8 da `SPEC-D-014` conferidos por comando na mesma data.
