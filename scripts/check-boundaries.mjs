/**
 * Gate de fronteiras do desktop.
 *
 * Oito regras arquiteturais que estão em prosa no `AGENTS.md` viram código
 * executável. Sem isto, "domain/ nunca importa ui/" é opinião, e opinião não
 * falha em review.
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

const ARQUIVO_RUST = "src-tauri/src/commands.rs";
const COMANDOS_RS = join(RAIZ, ARQUIVO_RUST);

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
 * As escritas (INSERT/UPDATE/DELETE) de cada `pub fn`, como
 * `Map<função, Set<dono>>`. O módulo de teste é cortado antes: teste escreve
 * para provar regra, não dado da usuária. O corte é no `#[cfg(test)] mod` —
 * **não** no primeiro `#[cfg(test)]`, porque este também marca helpers que só
 * o teste usa, no meio do arquivo, e cortar ali deixaria quase todo o backend
 * fora do gate (medido em F8b2: o gate "verde" que não via escrita nenhuma).
 */
function escritasPorFuncao(fonte) {
  const corte = fonte.search(/^#\[cfg\(test\)\]\s*\r?\nmod\s+\w+\s*\{/m);
  const corpo = semComentariosDeLinha(corte === -1 ? fonte : fonte.slice(0, corte));
  const limites = [...corpo.matchAll(/^pub fn (\w+)[(< ]/gm)].map((m) => ({
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
    if (dono === undefined) continue; // tabela sem dono é falha da regra, abaixo
    // O comando tauri é a casca `X`; a escrita mora em `X_core`. Sem normalizar
    // o sufixo, `donos.get("X_core")` é `undefined` e o cruzamento de D67 fica
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

{
  // 8a — tabela sem dono classificado, e mapa sem tabela: a checagem nas duas
  // direções é o que impede o mapa de envelhecer em silêncio (D65: a ausência
  // da linha é falha de aceite).
  for (const tabela of tabelas) {
    if (!(tabela in DONO_DA_TABELA)) {
      falha(
        COMANDOS_RS,
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
        COMANDOS_RS,
        1,
        "dono-de-dado",
        `o mapa do gate diz que \`${tabela}\` é de \`${DONO_DA_TABELA[tabela]}\`, ` +
          `mas a tabela não existe no schema. Mapa velho é a matriz mentindo.`,
      );
    }
  }

  // 8b — o cruzamento de D67: módulo da bridge × tabelas escritas.
  const escritas = escritasPorFuncao(readFileSync(COMANDOS_RS, "utf8"));
  const donos = donosPorComando();

  for (const [comando, donosDaBridge] of donos) {
    const modulos = [...donosDaBridge];
    if (modulos.length > 1) {
      falha(
        COMANDOS_RS,
        1,
        "dono-de-dado",
        `o comando \`${comando}\` é invocado pelas bridges de ${modulos.map((m) => `\`${m}\``).join(" e ")} — ` +
          `dado tem exatamente um dono (D65), e caminho de escrita com dois donos ` +
          `é dado de dois módulos.`,
      );
    }
  }

  for (const [funcao, donosEscritos] of escritas) {
    if (donosEscritos.size > 1) {
      falha(
        COMANDOS_RS,
        1,
        "dono-de-dado",
        `\`${funcao}\` escreve em tabelas de ${[...donosEscritos].map((d) => `\`${d}\``).join(" e ")} — ` +
          `nenhum comando de um módulo escreve em tabela de outro (D67).`,
      );
      continue;
    }
    const donoEscrito = [...donosEscritos][0];
    if (donoEscrito === undefined) continue;
    // `local` (setting) é escrita da casca em nome de qualquer módulo; o que
    // não pode é dado de módulo no comando de outro módulo.
    if (donoEscrito === "local") continue;
    const moduloDaBridge = donos.get(funcao);
    const modulo = moduloDaBridge ? [...moduloDaBridge][0] : null;
    if (modulo !== null && modulo !== donoEscrito) {
      falha(
        COMANDOS_RS,
        1,
        "dono-de-dado",
        `\`${funcao}\` escreve em tabela de \`${donoEscrito}\`, mas é invocado pela ` +
          `bridge de \`${modulo}\` — comando de um módulo não grava dado de outro (D67).`,
      );
    }
  }
}

if (problemas.length === 0) {
  console.log(
    `✓ fronteiras ok: ${arquivos.length} arquivos, ${tabelas.size} tabelas do backend ` +
      `verificadas contra ${ALVO}/ (dono de dado: ${Object.keys(DONO_DA_TABELA).length} tabelas classificadas)`,
  );
  process.exit(0);
}

console.error(`✗ ${problemas.length} violação(ões) de fronteira:\n`);
for (const p of problemas) {
  console.error(`  ${p.arquivo}:${p.linha}  [${p.regra}]`);
  console.error(`    ${p.mensagem}\n`);
}
process.exit(1);
