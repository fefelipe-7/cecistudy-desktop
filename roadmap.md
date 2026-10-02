# Roadmap

## Feito

- [x] Migrar a stack para React + Vite + npm e remover o framework SSR com Nitro.
- [x] Empacotar como app desktop com Tauri v2.
- [x] Remover código morto: server entry, roteador, telemetria e 28 componentes shadcn não usados.
- [x] Zerar os 2 warnings de `react-refresh/only-export-components` do baseline.
- [x] Corrigir os defeitos do shell: busca rápida com focus trap, controles inertes, checkbox sem estado acessível, tarefa duplicada no Kanban, contadores divergentes, ARIA inválido e `<a href="/">` no `ErrorBoundary`.
- [x] Fundação do módulo Calendário (`docs/calendario/00-fundacao.md`): decisões §12 fechadas, `src/features/calendar/{domain,data,ui,sync}/` criada, `src/lib/ipc.ts` e o vocabulário de camadas e níveis.

## Próximo

- [ ] Habilitar o modo escuro — o bloco `.dark` existe em `src/styles.css`, falta o controle que alterna a classe.
- [x] Domínio do Calendário escrito (`docs/calendario/01-dominio-persistencia.md`): tipos, máquina de estados, recorrência por RRULE, camada SQLite com 12 tabelas e os 20 comandos Tauri.
- [x] **`cargo` destravado e gate real** (2026-10-02). O bloqueio era `libwebkit2gtk-4.1-dev` ausente no Linux, **não** MSVC — o item do MSVC abaixo estava diagnosticado errado. `cargo check` e `cargo test` rodam, e foi assim que apareceu que **o backend Rust nunca compilou**: 8 erros de compilação (código e testes) e 2 bugs de produção (o filtro de `list_responsibilities` devolvia zero linhas; `FREQ=MONTHLY` e `WEEKLY` sem `BYDAY` expandiam errados). 28 testes Rust + 173 TypeScript verdes.
- [x] **Gate verificável** (2026-10-02): `npm run test`, `test:rust`, `lint:rust` (clippy `-D warnings`), `fmt:rust`, `gate:boundary` (fronteiras como código) e `npm run gate` que encadeia tudo. `.github/workflows/ci.yml` roda os sete passos. Antes havia 195 testes e nenhum comando que os rodasse.
- [ ] Instalar as MSVC Build Tools 2022 com o workload "Desktop development with C++" — **só no Windows**. Em Linux o que falta é `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libssl-dev` e `libxdo-dev` (o CI já instala). O diagnóstico completo está em `SPEC.md`.
- [x] Estrutura das cinco visões do Calendário (D15bis) e a geometria da grade (`domain/grid-scale.ts`, escala única `timeToY`/`yToTime`).
- [ ] Arraste, redimensionamento, criação e edição na grade — Fase 2.4 da spec 02, precisa do app de pé.
- [ ] Corrigir o overflow horizontal abaixo de ~480px de largura.
- [ ] Ícone dedicado do produto — hoje os ícones do app vêm do favicon de 20 KB.
- [ ] Auto-hospedar a Inter (hoje é uma requisição ao Google Fonts a cada launch).
