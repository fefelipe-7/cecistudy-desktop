# Onde estas citações de código apontam

> **Aviso de 2026-10-15.** Os documentos desta pasta citam `commands.rs:NNN`,
> `db.rs` e `domain.rs`. **Esses caminhos não existem mais**: em
> 2026-10-15 a `SPEC-D-015` (`D96..D100`) trocou o caminho de escrita monolítico
> por um diretório por dono de dado, e a correspondência é:
>
> | Antes | Agora |
> |---|---|
> | `src-tauri/src/commands.rs` | `src-tauri/src/<dono>/comandos.rs` — `calendario`, `faculdade`, `estudos` ou `tcc` |
> | `src-tauri/src/db.rs` | `src-tauri/src/nucleo/db.rs` |
> | `src-tauri/src/domain.rs` | `src-tauri/src/calendario/dominio.rs` |
>
> **Nenhum número de linha foi herdado.** A migração moveu 6537 linhas de
> endereço; a linha `NNN` que uma citação fazia não diz mais nada. Para
> reapontar, o caminho é o diretório do dono e o número é **medido** com o
> `rg` no arquivo de hoje.
>
> **O que continua valendo aqui:** as decisões `D1..D46` de `00-fundacao.md`, que
> são a origem da governança do desktop e o melhor formato de decisão do grupo,
> e as invariantes `INV-1..INV-7`. Elas descrevem **regras**, e regra não mudou
> de arquivo — mudou de lugar.
>
> **O que não continua valendo:** qualquer afirmação sobre *onde* o código estava
> quando estas specs foram escritas (2026-09-28/29), e os números de linha.
> Onde estas specs e [`docs/espacos/`](../espacos/) divergirem sobre regra,
> vale `docs/espacos/`; onde divergirem sobre caminho, vale o código.
>
> A dívida de reapontar está registrada em
> [`docs/todo/TO-DO-DESKTOP.md`](../todo/TO-DO-DESKTOP.md) §Bloco B.

---