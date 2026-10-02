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
- [ ] Instalar as MSVC Build Tools 2022 com o workload "Desktop development with C++" — sem isso **nem `cargo check` roda**, porque build scripts e proc-macros são linkados para o host (bloqueia as specs 01, 02 e 05). O diagnóstico completo, com a alternativa mingw-w64, está em `SPEC.md`.
- [x] Domínio do Calendário escrito (`docs/calendario/01-dominio-persistencia.md`): tipos, máquina de estados, recorrência por RRULE, camada SQLite com 12 tabelas e os 20 comandos Tauri. 157 testes de domínio verdes e schema validado contra um engine SQLite real; falta `cargo test` e `tauri:dev` por causa do item acima.
- [x] Estrutura das cinco visões do Calendário (D15bis) e a geometria da grade (`domain/grid-scale.ts`, escala única `timeToY`/`yToTime`).
- [ ] Arraste, redimensionamento, criação e edição na grade — Fase 2.4 da spec 02, precisa do app de pé.
- [ ] Corrigir o overflow horizontal abaixo de ~480px de largura.
- [ ] Ícone dedicado do produto — hoje os ícones do app vêm do favicon de 20 KB.
- [ ] Auto-hospedar a Inter (hoje é uma requisição ao Google Fonts a cada launch).
