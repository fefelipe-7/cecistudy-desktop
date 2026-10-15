/**
 * Gate de fronteiras do desktop.
 *
 * Dez regras arquiteturais que estão em prosa no `AGENTS.md` viram código
 * executável. Sem isto, "domain/ nunca importa ui/" é opinião, e opinião não
 * falha em review.
 *
 * Sete regras cuidam do TypeScript. A oitava é o dono de dado no backend Rust
 * (SPEC-D-007 D67) e a nona e a décima são a modularidade do crate
 * (SPEC-D-015, D96..D100).
 *
 * A regra 8 percorre `src-tauri/src` recursivamente e tira o dono de cada
 * escrita do **nome do diretório**. Antes ela lia `src-tauri/src/commands.rs`
 * por nome, e isso significava que dividir aquele arquivo — que era justamente
 * o que D96 mandava fazer — deixava a regra sem ver escrita nenhuma e ainda
 * verde. Verificado em 2026-10-14: o gate acusava a violação antes do split e
 * dava `✓` depois.
 *
 * A dívida que existiu entre F0 e F4 — o caminho de escrita morando na raiz do
 * crate — morava em `EXCECOES_NA_RAIZ`, nomeada e impressa em cada execução. O
 * arquivo sumiu em F5, e a exceção foi com ele: a regra fica mais dura do que o
 * estado que ela mede.
 *
 * A verificação negativa é `npm run test:gate`: sem ela, "o gate está verde"
 * não distingue "está certo" de "não viu nada".
 *
 * Sem dependência: só `node:*`. Roda em segundos e não precisa de `npm install`.
 *
 * Uso: `npm run check:boundaries`
 * Em falha → exit 1, com `caminho:linha` e o que fazer.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = fileURLToPath(new URL("..", import.meta.url));

/** Arquivos que o gate lê. Vite/Tauri não entra: o gate é sobre `src/`. */
const ALVO = "src";
const EXTENSOES = [".ts", ".tsx"];

/** `scripts/` e `*.test.ts` não são código de produção e não são fronteira. */
function ehTeste(caminho) {
  return caminho.includes(`.test.`) || caminho.includes(`__tests__${sep}`);
}

/** Lista recursiva de arquivos sob `src/`. */
function listar(diretorio, saida = []) {
  for (const entrada of readdirSync(diretorio, { withFileTypes: true })) {
    const completo = join(diretorio, entrada.name);
    if (entrada.isDirectory()) {
      listar(completo, saida);
    } else if (EXTENSOES.some((ext) => entrada.name.endsWith(ext))) {
      saida.push(completo);
    }
  }
  return saida;
}

/**
 * Cada import de um arquivo, como `{ linha, modulo, tipoImport }`.
 *
 * `import type` conta: a dependência invertida do `packages/navigation` do
 * mobile era só de tipo, e o gate de lá não pegava. Aqui também conta.
 */
function importsDe(caminho) {
  const fonte = readFileSync(caminho, "utf8");
  const linhas = fonte.split(/\r?\n/);
  const achados = [];

  for (let i = 0; i < linhas.length; i++) {
    // Import multilinha: junta a partir desta linha até achar o `from`.
    let trecho = linhas[i];
    let fim = i;
    while (from(trecho) === null && fim + 1 < linhas.length && fim - i < 30) {
      fim++;
      trecho += `\n${linhas[fim]}`;
    }
    const achado = from(trecho);
    if (achado) {
      achados.push({
        linha: i + 1,
        modulo: achado.modulo,
        tipoImport: achado.tipo,
      });
      i = fim;
    }
  }
  return achados;

  function from(texto) {
    const m = /(?:^|\s)(?:import|export)\s+(type\s+)?(?:[\s\S]*?)\s*from\s+["']([^"']+)["']/.exec(
      texto,
    );
    return m ? { tipo: Boolean(m[1]), modulo: m[2] } : null;
  }
}

/** Resolve um especificador relativo a partir do arquivo, em caminho de repo. */
function resolver(arquivo, modulo) {
  if (!modulo.startsWith(".")) return null;
  // `arquivo` já é absoluto e `join` **não** reseta em caminho absoluto (ao
  // contrário de `resolve`), então `RAIZ` aqui duplicaria o prefixo e nada
  // resolveria. O caminho relativo sai do `relative` do fim.
  const base = join(arquivo, "..", modulo);
  const candidatos = [
    base,
    ...EXTENSOES.map((e) => `${base}${e}`),
    ...EXTENSOES.map((e) => join(base, `index${e}`)),
  ];
  for (const c of candidatos) {
    try {
      if (statSync(c).isFile()) return relative(RAIZ, c).split(sep).join("/");
    } catch {
      // não existe; segue
    }
  }
  return null;
}

const problemas = [];
function falha(arquivo, linha, regra, mensagem) {
  problemas.push({ arquivo: relative(RAIZ, arquivo).split(sep).join("/"), linha, regra, mensagem });
}

/**
 * Módulo dono de um caminho `src/features/<módulo>/…`, ou `null` fora de
 * `features/`. É a chave da regra 7 (`fronteira-de-modulo`, SPEC-D-007 D67).
 */
function moduloDe(rel) {
  return /^src\/features\/([^/]+)\//.exec(rel)?.[1] ?? null;
}

const arquivos = listar(join(RAIZ, ALVO)).filter((f) => !ehTeste(f));

/**
 * Nomes de tabela do schema do backend. Nenhum deles pode aparecer em `src/`.
 *
 * A fonte é a verdade: `src-tauri/migrations/*.sql`. Se alguém criar tabela nova
 * lá, ela entra na lista pela varredura — não por memória.
 */
function tabelasDoSchema() {
  const dir = join(RAIZ, "src-tauri", "migrations");
  const tabelas = new Set();
  let arquivosSql;
  try {
    arquivosSql = readdirSync(dir).filter((n) => n.endsWith(".sql"));
  } catch {
    return tabelas;
  }
  for (const nome of arquivosSql) {
    const sql = readFileSync(join(dir, nome), "utf8");
    for (const m of sql.matchAll(/CREATE TABLE(?: IF NOT EXISTS)?\s+(\w+)/gi)) {
      tabelas.add(m[1].toLowerCase());
    }
  }
  return tabelas;
}

const tabelas = tabelasDoSchema();

for (const arquivo of arquivos) {
  const rel = relative(RAIZ, arquivo).split(sep).join("/");
  const ehDomain = rel.startsWith("src/features/calendar/domain/");
  const ehLib = rel.startsWith("src/lib/");

  for (const imp of importsDe(arquivo)) {
    const destino = resolver(arquivo, imp.modulo);
    if (!destino) continue;

    // Regra 1 — `domain/` é puro: não importa `ui/`, `data/` nem `sync/`.
    if (ehDomain) {
      const proibido = ["calendar/ui/", "calendar/data/", "calendar/sync/"];
      if (proibido.some((p) => destino.includes(p))) {
        falha(
          arquivo,
          imp.linha,
          "domain-puro",
          `domain/ não pode importar ${destino}. A regra é do AGENTS.md:17 — ` +
            `domain/ fica puro para ser testável em toda máquina.`,
        );
      }
    }

    // Regra 2 — `domain/` também não entra na lib de plataforma.
    if (ehDomain && destino === "src/lib/ipc.ts") {
      falha(
        arquivo,
        imp.linha,
        "domain-puro",
        "domain/ não pode importar src/lib/ipc.ts: é o ponto de entrada do Tauri.",
      );
    }

    // Regra 3 — `domain/` não entra em React nem em Capacitor.
    if (ehDomain && /from\s+["'](react|@tauri-apps|@capacitor)/.test(`from "${imp.modulo}"`)) {
      falha(
        arquivo,
        imp.linha,
        "domain-puro",
        `domain/ não pode importar ${imp.modulo}: domain/ é puro, sem React e sem glue de plataforma.`,
      );
    }

    // Regra 6 — os quatro espelhos de regra não têm chamador de produção fora
    // de `domain/`. A regra é do Rust (`src-tauri/src/domain.rs`); o espelho
    // existe para tipar e para o teste que roda sem o toolchain Rust. Com o
    // read-model derivado pelo backend (`GridItem.allowedTransitions`,
    // `Responsibility.isOverdue`) e o concluir passando por `complete_item`,
    // `ui/`, `data/` e `sync/` não precisam re-derivar nada.
    const ehProducaoDaGrade =
      rel.startsWith("src/features/calendar/ui/") ||
      rel.startsWith("src/features/calendar/data/") ||
      rel.startsWith("src/features/calendar/sync/");
    const espelhos = [
      "src/features/calendar/domain/state-machine.ts",
      "src/features/calendar/domain/recurrence.ts",
      "src/features/calendar/domain/overdue.ts",
      "src/features/calendar/domain/overlap.ts",
    ];
    if (ehProducaoDaGrade && espelhos.includes(destino)) {
      falha(
        arquivo,
        imp.linha,
        "regra-no-rust",
        `importa o espelho ${destino} no caminho de produção. A regra é do Rust ` +
          `(src-tauri/src/domain.rs); o espelho existe para teste. Use o read-model ` +
          `(GridItem.allowedTransitions, Responsibility.isOverdue) ou o comando do backend.`,
      );
    }

    // Regra 7 — fronteira de módulo: a `ui/` de um módulo não importa a `data/`
    // de outro. O dono do dado é uma linha só (SPEC-D-007 D65), e quem só
    // mostra o dado de outro módulo o lê por referência viva no backend (D66),
    // nunca importando a camada de persistência alheia no componente.
    const origemModulo = moduloDe(rel);
    const destinoModulo = moduloDe(destino);
    if (
      /^src\/features\/[^/]+\/ui\//.test(rel) &&
      /^src\/features\/[^/]+\/data\//.test(destino) &&
      origemModulo !== destinoModulo
    ) {
      falha(
        arquivo,
        imp.linha,
        "fronteira-de-modulo",
        `ui/ do módulo \`${origemModulo}\` importa data/ do módulo \`${destinoModulo}\`. ` +
          `Dado de outro módulo é referência viva no backend (SPEC-D-007 D66/D67), ` +
          `nunca import direto da camada de dado alheia.`,
      );
    }
  }

  const fonte = readFileSync(arquivo, "utf8");

  // Regra 4 — só `src/lib/ipc.ts` obtém `invoke` do Tauri.
  //
  // O que se verifica é a **origem do import**, não a chamada: `data/bridge.ts`
  // legitimately chama `invoke`, porque ele importa de `lib/ipc.ts`. O que a
  // regra proíbe é alguém importar `invoke` do Tauri por fora de `lib/ipc.ts` —
  // aí a fronteira vira dois caminhos e ninguém sabe mais por onde passa a IPC.
  for (const imp of importsDe(arquivo)) {
    const vemDoTauri = imp.modulo.startsWith("@tauri-apps/api");
    const ehIpc = rel === "src/lib/ipc.ts";
    if (vemDoTauri && !ehIpc) {
      falha(
        arquivo,
        imp.linha,
        "ipc-unico",
        `importa ${imp.modulo} fora de src/lib/ipc.ts. Toda IPC passa por ipc.ts — ` +
          `é o que torna a fronteira entre Rust e TypeScript auditável com um rg.`,
      );
    }
  }

  // Regra 5 — nenhum SQL atravessa a fronteira para o TypeScript.
  //
  // Não basta o **nome** da tabela aparecer: `responsibility` é conceito de
  // domínio e aparece legitimamente em `ui/` e em `domain/`. O que não pode
  // atravessar é **SQL escrito no cliente** — e o sinal disso é um verbo de
  // statement junto com uma tabela do schema. Um gate que acusa o nome sozinho
  // dá falso positivo, e gate que dá falso positivo é desligado por quem o vê
  // falhar em código certo.
  const SQL_STATEMENT = /\b(SELECT\s|INSERT\s+INTO\b|UPDATE\s+\w+\s+SET\b|DELETE\s+FROM\b)/i;
  for (const [i, linha] of fonte.split(/\r?\n/).entries()) {
    if (!SQL_STATEMENT.test(linha)) continue;
    for (const tabela of tabelas) {
      const re = new RegExp(`\\b${tabela}\\b`, "i");
      if (re.test(linha)) {
        falha(
          arquivo,
          i + 1,
          "sql-nao-atravessa",
          `SQL no cliente sobre a tabela \`${tabela}\`. O AGENTS.md:16 diz que ` +
            `\`data/\` conhece o schema e não o SQL; a query é do Rust.`,
        );
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Regra 8 — dono de dado no backend (SPEC-D-007, D67)
//
// "Um módulo Rust por dono de dado, e nenhum `#[tauri::command]` de um módulo
// escreve em tabela de outro — a verificação cruza a contagem de comandos com
// a lista de D65."
//
// O cruzamento é nas duas pontas: o **módulo dono** de cada comando vem da
// bridge que o invoca (`src/features/<módulo>/data/`), e as **tabelas
// escritas** vêm do corpo das funções `pub fn` de `src-tauri/src/*.rs`. As
// escritas de cada função pertencem ao trecho entre um `pub fn` e o próximo —
// helpers incluídos, porque helper que escreve é caminho de escrita também.
//
// Isenções, por desenho:
// - `db.rs` e `migrations/`: schema e seed são bootstrap, não comando de
//   módulo. O gate de lá é a contagem de tabelas do teste do `db.rs`.
// - `#[cfg(test)]`: o módulo de teste escreve para provar regra, não para
//   gravar dado da usuária.
// - Leituras (SELECT) não são propriedade: "quem só mostra" é referência
//   viva (D66), e ler a tabela de outro módulo é o modo de vínculo.
// ---------------------------------------------------------------------------

/**
 * Pasta de feature → id de módulo da matriz D65. O Calendário mora em
 * `src/features/calendar/` (nome legado da pasta); o id do módulo, na matriz e
 * na navegação, é `calendario`. Pastas que já batem com o id não precisam de
 * linha aqui.
 */
const DONO_DO_DIRETORIO = { calendar: "calendario" };

/** A tradução tabela→dono da matriz D65 (`docs/espacos/01-fronteiras.md`). */
const DONO_DA_TABELA = {
  // Calendário (spec 01 §3.3 + integrações da spec 05)
  layer: "calendario",
  calendar_event: "calendario",
  event_recurrence: "calendario",
  event_occurrence: "calendario",
  responsibility: "calendario",
  responsibility_step: "calendario",
  plan_block: "calendario",
  execution_record: "calendario",
  suggestion: "calendario",
  external_link: "calendario",
  sync_cursor: "calendario",
  // Faculdade (SPEC-D-010, D74/D75)
  semestre: "faculdade",
  disciplina: "faculdade",
  avaliacao: "faculdade",
  nota: "faculdade",
  regra_padrao: "faculdade",
  regra_disciplina: "faculdade",
  regra_congelada: "faculdade",
  // Estudos (SPEC-D-010, D76)
  conteudo: "estudos",
  cartao: "estudos",
  revisao: "estudos",
  // TCC & Projetos (SPEC-D-013, D85..D88)
  projeto: "tcc",
  documento: "tcc",
  no_documento: "tcc",
  inline_run: "tcc",
  versao: "tcc",
  item_a_resolver: "tcc",
  base_de_pesquisa: "tcc",
  saida: "tcc",
  // Preferência local — a matriz D65 diz "sem dono de módulo"
  setting: "local",
};

// ---------------------------------------------------------------------------
// O lado Rust: dono de dado, registro e teto (SPEC-D-007 D67, SPEC-D-015)
// ---------------------------------------------------------------------------

/** Base do crate Rust. A regra 8, a 9 e a 10 leem daqui. */
const SRC_RUST = join(RAIZ, "src-tauri", "src");

/** O próprio gate: a causa de 8a é o mapa dele, não um arquivo do app. */
const ESTE_GATE = join(RAIZ, "scripts", "check-boundaries.mjs");

/**
 * Diretórios de módulo Rust, e o nome do diretório **é** o id do dono (D97,
 * I1). Um diretório fora desta lista é violação: é a mesma disciplina de D65,
 * onde tabela sem classificação é falha de aceite. Sem isso, um módulo novo
 * nasce invisível.
 */
const DONOS_RUST = new Set(["calendario", "faculdade", "estudos", "tcc"]);

/**
 * Diretório sem dono de dado: a infraestrutura que os quatro módulos
 * compartilham — `Db`, `DbError`, `new_id`, `now_ms` (D96).
 */
const SEM_DONO_RUST = new Set(["nucleo"]);

/**
 * Teto de linhas de `<módulo>/comandos.rs` (D100, I5).
 */
const TETO_DE_COMANDOS = 2000;

/** Todos os `.rs` de `src-tauri/src/`. */
function arquivosRust() {
  const saida = [];
  const anda = (dir) => {
    for (const entrada of readdirSync(dir, { withFileTypes: true })) {
      const completo = join(dir, entrada.name);
      if (entrada.isDirectory()) anda(completo);
      else if (entrada.name.endsWith(".rs")) saida.push(completo);
    }
  };
  anda(SRC_RUST);
  return saida;
}

/**
 * O dono de um arquivo Rust: a primeira parte do caminho, quando for diretório
 * de módulo. `null` = sem dono — é o caso da raiz do crate e de `nucleo/`.
 */
function donoDoArquivoRust(rel) {
  const pasta = /^src-tauri\/src\/([^/]+)\//.exec(rel)?.[1];
  return DONOS_RUST.has(pasta) ? pasta : null;
}

/** Remove comentários de linha, exceto quando o `//` está dentro de string. */
function semComentariosDeLinha(fonte) {
  return fonte
    .split(/\r?\n/)
    .map((linha) => {
      const idx = linha.indexOf("//");
      if (idx === -1) return linha;
      return linha.slice(0, idx).includes('"') ? linha : linha.slice(0, idx);
    })
    .join("\n");
}

/**
 * O corpo de produção de um arquivo: sem o `#[cfg(test)] mod`.
 *
 * O corte é no `mod`, **nunca** no primeiro `#[cfg(test)]`, porque esse também
 * marca helpers que só o teste usa no meio do arquivo, e cortar ali deixaria
 * quase todo o backend fora do gate (medido em F8b2: o gate "verde" que não via
 * escrita nenhuma). Com D99 os testes vão para `<módulo>/testes.rs`, e o corte
 * passa a ser redundante por construção — mas ele continua, porque `db.rs` tem
 * teste no mesmo arquivo.
 */
function corpoDeProducao(fonte) {
  const corte = fonte.search(/^#\[cfg\(test\)\]\s*\r?\nmod\s+\w+\s*\{/m);
  return semComentariosDeLinha(corte === -1 ? fonte : fonte.slice(0, corte));
}

/**
 * As escritas (INSERT/UPDATE/DELETE) de cada `fn` de UM arquivo, como
 * `Map<comando, Set<dono da tabela>>`.
 *
 * **Todo `fn` conta, público ou privado.** O privado continua sendo caminho de
 * escrita: a casca `pub` do mesmo módulo o chama, então ele é alcançável de
 * fora do crate. Já existe um assim — `upsert_recurrence_core` é privado entre
 * 47 públicos. Se o gate ignorasse `fn` privado, o verde falso de D98 voltaria
 * pela porta dos fundos.
 */
function escritasPorFuncaoDoArquivo(fonte) {
  const corpo = corpoDeProducao(fonte);
  const limites = [...corpo.matchAll(/^(?:pub\s+)?fn\s+(\w+)[(< ]/gm)].map((m) => ({
    nome: m[1],
    indice: m.index,
  }));
  const ESCRITA = /\b(?:INSERT\s+(?:OR\s+\w+\s+)?INTO|UPDATE|DELETE\s+FROM)\s+(\w+)/gi;
  const escritas = new Map();
  for (const m of corpo.matchAll(ESCRITA)) {
    let limite = null;
    for (const l of limites) {
      if (l.indice <= m.index) limite = l;
      else break;
    }
    if (limite === null) continue;
    const dono = DONO_DA_TABELA[m[1].toLowerCase()];
    if (dono === undefined) continue; // tabela sem dono é falha da regra, 8a
    // A casca tauri é `X`; a escrita mora em `X_core`. Sem normalizar o
    // sufixo, `donos.get("X_core")` é `undefined` e o cruzamento de D67 fica
    // cego — foi o defeito medido em F8b2. O nome que a bridge invoca é `X`.
    const comando = limite.nome.endsWith("_core")
      ? limite.nome.slice(0, -"_core".length)
      : limite.nome;
    if (!escritas.has(comando)) escritas.set(comando, new Set());
    escritas.get(comando).add(dono);
  }
  return escritas;
}

/**
 * O módulo dono de cada comando, pela bridge que o invoca. Comando invocado
 * por duas bridges de módulos diferentes é dado com dois donos — e a lista de
 * D65 diz que dono é um só.
 */
function donosPorComando() {
  const donos = new Map();
  for (const arquivo of arquivos) {
    const rel = relative(RAIZ, arquivo).split(sep).join("/");
    const pasta = /^src\/features\/([^/]+)\/data\//.exec(rel)?.[1];
    if (!pasta) continue;
    const modulo = DONO_DO_DIRETORIO[pasta] ?? pasta;
    const fonte = readFileSync(arquivo, "utf8");
    for (const m of fonte.matchAll(/invoke(?:<[^>]*>)?\(\s*"([^"]+)"/g)) {
      if (!donos.has(m[1])) donos.set(m[1], new Set());
      donos.get(m[1]).add(modulo);
    }
  }
  return donos;
}

/** Núcleos (`X_core`) e cascas (`#[tauri::command] fn X`) do crate inteiro. */
function funcoesDeIpc() {
  const nucleos = new Map();
  const cascas = new Map();
  for (const caminho of arquivosRust()) {
    const rel = relative(RAIZ, caminho).split(sep).join("/");
    const corpo = corpoDeProducao(readFileSync(caminho, "utf8"));
    for (const m of corpo.matchAll(/^(?:pub\s+)?fn\s+(\w+_core)\b/gm)) {
      nucleos.set(m[1].slice(0, -"_core".length), rel);
    }
    // O atributo vem em duas linhas: `cfg_attr(feature, tauri::command)` e
    // `cfg(feature)`, e pode haver outros `#[...]` entre o atributo e a `fn`.
    for (const m of corpo.matchAll(
      /tauri::command\)\]\s*\r?\n(?:#\[[^\]]*\]\s*\r?\n)*(?:pub\s+)?fn\s+(\w+)\s*\(/g,
    )) {
      cascas.set(m[1], rel);
    }
  }
  return { nucleos, cascas };
}

/**
 * Os comandos registrados em `lib.rs`, com a linha, para acusar duplicata.
 *
 * Aceita o caminho completo do módulo (`calendario::comandos::migrate`) e
 * guarda só o último segmento, que é o nome que o `invoke` usa. O caminho
 * importa para a leitura de quem abre o arquivo; para o cruzamento, o que
 * identifica o comando é o nome.
 */
function comandosRegistrados() {
  const lib = readFileSync(join(SRC_RUST, "lib.rs"), "utf8");
  const registrados = new Map();
  const linhas = lib.split(/\r?\n/);
  for (let i = 0; i < linhas.length; i++) {
    // Só a forma `mod::…::comando`, que é como `generate_handler!` lista.
    const m = /^\s*(?:\w+::)+\w+,?\s*$/.exec(linhas[i]);
    if (!m) continue;
    const nome = m[0].trim().replace(/,$/, "").split("::").pop();
    if (!registrados.has(nome)) registrados.set(nome, []);
    registrados.get(nome).push(i + 1);
  }
  return registrados;
}

/** Dívida da raiz (D98): arquivo da raiz do crate → módulos que escrevem dali. */
const dividaNaRaiz = new Map();

{
  // 8a — tabela sem dono classificado, e mapa sem tabela: a checagem nas duas
  // direções é o que impede o mapa de envelhecer em silêncio (D65: a ausência
  // da linha é falha de aceite).
  for (const tabela of tabelas) {
    if (!(tabela in DONO_DA_TABELA)) {
      falha(
        ESTE_GATE,
        1,
        "dono-de-dado",
        `a tabela \`${tabela}\` não tem dono no mapa do gate. Classifique-a na ` +
          `matriz D65 (docs/espacos/01-fronteiras.md) e em DONO_DA_TABELA — ` +
          `tabela sem dono é a falha que a matriz proíbe.`,
      );
    }
  }
  for (const tabela of Object.keys(DONO_DA_TABELA)) {
    if (!tabelas.has(tabela)) {
      falha(
        ESTE_GATE,
        1,
        "dono-de-dado",
        `o mapa do gate diz que \`${tabela}\` é de \`${DONO_DA_TABELA[tabela]}\`, ` +
          `mas a tabela não existe no schema. Mapa velho é a matriz mentindo.`,
      );
    }
  }

  const donos = donosPorComando();

  for (const caminho of arquivosRust()) {
    const rel = relative(RAIZ, caminho).split(sep).join("/");
    const pasta = /^src-tauri\/src\/([^/]+)\//.exec(rel)?.[1];
    const donoDoArquivo = donoDoArquivoRust(rel);

    // Diretório de módulo sem dono declarado: o módulo novo nasce invisível.
    if (pasta !== undefined && !DONOS_RUST.has(pasta) && !SEM_DONO_RUST.has(pasta)) {
      falha(
        caminho,
        1,
        "dono-de-dado",
        `\`${pasta}/\` não é um módulo dono declarado. Acrescente-o a DONOS_RUST ` +
          `(\`${pasta}\`) ou a SEM_DONO_RUST, e a tabela dele a DONO_DA_TABELA ` +
          `(docs/espacos/01-fronteiras.md).`,
      );
      continue;
    }

    // **Todo arquivo que escreve mora num diretório de módulo**, e o nome do
    // diretório é o id do dono (D96, I1). Não há exceção: um arquivo de escrita
    // na raiz do crate é violação, e é assim desde F5.
    //
    // A dívida que existiu entre F0 e F4 morava em `EXCECOES_NA_RAIZ`, uma lista
    // nomeada e impressa em toda execução. Ela foi removida junto com o arquivo
    // que a justificava — `commands.rs` não existe mais, e a regra precisa ficar
    // mais dura do que o estado que ela mede.
    if (pasta === undefined) {
      const nome = rel.slice("src-tauri/src/".length);
      const escreve = escritasPorFuncaoDoArquivo(readFileSync(caminho, "utf8"));
      const tabelasDeModulo = new Set();
      for (const donosEscritos of escreve.values()) {
        for (const d of donosEscritos) if (DONOS_RUST.has(d)) tabelasDeModulo.add(d);
      }
      if (tabelasDeModulo.size > 0) {
        falha(
          caminho,
          1,
          "dono-de-dado",
          `\`${nome}\` está na raiz do crate e escreve em tabela de ` +
            `${[...tabelasDeModulo].map((d) => `\`${d}\``).join(", ")}. Todo arquivo ` +
            `que escreve mora num diretório de módulo, e o nome do diretório é o ` +
            `id do dono (D96, I1). A exceção que existia durante a migração foi ` +
            `removida em F5.`,
        );
      }
    }

    for (const [comando, donosEscritos] of escritasPorFuncaoDoArquivo(
      readFileSync(caminho, "utf8"),
    )) {
      if (donosEscritos.size > 1) {
        falha(
          caminho,
          1,
          "dono-de-dado",
          `\`${comando}\` escreve em tabelas de ${[...donosEscritos].map((d) => `\`${d}\``).join(" e ")} — ` +
            `nenhum comando de um módulo escreve em tabela de outro (D67).`,
        );
        continue;
      }
      const donoEscrito = [...donosEscritos][0];
      if (donoEscrito === undefined) continue;
      // `local` (setting) é escrita da casca em nome de qualquer módulo; o que
      // não pode é dado de módulo no comando de outro módulo.
      if (donoEscrito === "local") continue;

      if (donoDoArquivo !== null && donoDoArquivo !== donoEscrito) {
        // A primeira metade de D67, agora verificável: o dono do arquivo é o
        // dono do dado, e é o diretório que diz isso.
        falha(
          caminho,
          1,
          "dono-de-dado",
          `\`${comando}\` mora em \`${donoDoArquivo}/\` e escreve em tabela de ` +
            `\`${donoEscrito}\` — o caminho de escrita de um dado mora no módulo ` +
            `dele (D96).`,
        );
        continue;
      }

      const moduloDaBridge = donos.get(comando);
      const modulo = moduloDaBridge ? [...moduloDaBridge][0] : null;
      if (modulo !== null && modulo !== donoEscrito) {
        falha(
          caminho,
          1,
          "dono-de-dado",
          `\`${comando}\` escreve em tabela de \`${donoEscrito}\`, mas é invocado pela ` +
            `bridge de \`${modulo}\` — comando de um módulo não grava dado de outro (D67).`,
        );
      }
    }
  }

  // Comando invocado por duas bridges de módulos diferentes.
  for (const [comando, donosDaBridge] of donos) {
    const modulos = [...donosDaBridge];
    if (modulos.length > 1) {
      falha(
        ESTE_GATE,
        1,
        "dono-de-dado",
        `o comando \`${comando}\` é invocado pelas bridges de ${modulos.map((m) => `\`${m}\``).join(" e ")} — ` +
          `dado tem exatamente um dono (D65), e caminho de escrita com dois donos ` +
          `é dado de dois módulos.`,
      );
    }
  }

  // Regra 9 — registro × função (D97, I4). O registro é a fronteira entre Rust
  // e TypeScript: um nome que diverge só falha em runtime, na tela.
  {
    const { nucleos, cascas } = funcoesDeIpc();
    const registrados = comandosRegistrados();
    for (const [nome, onde] of nucleos) {
      if (!cascas.has(nome)) {
        falha(
          join(RAIZ, onde),
          1,
          "registro-de-comando",
          `\`${nome}_core\` não tem casca \`#[tauri::command] fn ${nome}\`. Ou a ` +
            `casca foi removida e o núcleo virou código morto, ou a casca está com ` +
            `outro nome.`,
        );
      }
    }
    for (const [nome, onde] of cascas) {
      if (!nucleos.has(nome)) {
        falha(
          join(RAIZ, onde),
          1,
          "registro-de-comando",
          `a casca \`${nome}\` não tem núcleo \`${nome}_core\`. A casca só ` +
            `desembrulha o State e delega (commands.rs:10-12).`,
        );
        continue;
      }
      if (!registrados.has(nome)) {
        falha(
          join(RAIZ, onde),
          1,
          "registro-de-comando",
          `a casca \`${nome}\` não está registrada em lib.rs. Sem registro, ` +
            `\`invoke("${nome}")\` falha em runtime e a tela não acusa.`,
        );
      }
    }
    for (const [nome, linhas] of registrados) {
      const LIB_RS = join(SRC_RUST, "lib.rs");
      if (!cascas.has(nome)) {
        falha(
          LIB_RS,
          linhas[0],
          "registro-de-comando",
          `lib.rs registra \`${nome}\`, que não é casca de nenhum comando. ` +
            `Ou a casca foi apagada, ou sobrou registro.`,
        );
      }
      if (linhas.length > 1) {
        falha(
          LIB_RS,
          linhas[1],
          "registro-de-comando",
          `\`${nome}\` está registrado duas vezes em lib.rs ` + `(linhas ${linhas.join(" e ")}).`,
        );
      }
    }
  }

  // Regra 10 — teto de `<módulo>/comandos.rs` (D100, I5). Vale só para o arquivo
  // que o split cria e que volta a crescer: `tcc/docx.rs` é o round-trip DOCX,
  // uma unidade coerente, e não é caminho de escrita.
  for (const caminho of arquivosRust()) {
    const rel = relative(RAIZ, caminho).split(sep).join("/");
    const modulo = /^src-tauri\/src\/([^/]+)\/comandos\.rs$/.exec(rel)?.[1];
    if (!modulo) continue;
    const linhas = readFileSync(caminho, "utf8").split(/\r?\n/).length;
    if (linhas > TETO_DE_COMANDOS) {
      falha(
        caminho,
        linhas,
        "teto-de-comandos",
        `\`${rel}\` tem ${linhas} linhas, acima do teto de ${TETO_DE_COMANDOS} ` +
          `(D100). A resposta é dividir por sub-recurso, não subir o teto: o ` +
          `dono do dado continua sendo \`${modulo}/\`.`,
      );
    }
  }
}

if (problemas.length === 0) {
  const modulosRust = arquivosRust().filter((c) =>
    /^src-tauri\/src\/[^/]+\/comandos\.rs$/.test(relative(RAIZ, c).split(sep).join("/")),
  );
  console.log(
    `✓ fronteiras ok: ${arquivos.length} arquivos, ${tabelas.size} tabelas do backend ` +
      `verificadas contra ${ALVO}/ (dono de dado: ${Object.keys(DONO_DA_TABELA).length} ` +
      `tabelas classificadas) · ${modulosRust.length} módulos Rust com caminho de escrita`,
  );
  process.exit(0);
}

console.error(`✗ ${problemas.length} violação(ões) de fronteira:\n`);
for (const p of problemas) {
  console.error(`  ${p.arquivo}:${p.linha}  [${p.regra}]`);
  console.error(`    ${p.mensagem}\n`);
}
process.exit(1);
