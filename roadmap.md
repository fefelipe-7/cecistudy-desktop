# Roadmap

## Feito

- [x] Migrar a stack para React + Vite + npm e remover o framework SSR com Nitro.
- [x] Empacotar como app desktop com Tauri v2.
- [x] Remover código morto: server entry, roteador, telemetria e 28 componentes shadcn não usados.

## Próximo

- [ ] Habilitar o modo escuro — o bloco `.dark` existe em `src/styles.css`, falta o controle que alterna a classe.
- [ ] Persistir estado no disco via `@tauri-apps/plugin-store` (hoje tudo vive em `useState` e some ao fechar).
- [ ] Substituir o `<div role="dialog">` da busca rápida pelo componente `Dialog` do curated set (foco e trap de teclado).
- [ ] Corrigir o overflow horizontal abaixo de ~480px de largura.
- [ ] Ícone dedicado do produto — hoje os ícones do app vêm do favicon de 20 KB.
- [ ] Auto-hospedar a Inter (hoje é uma requisição ao Google Fonts a cada launch).
