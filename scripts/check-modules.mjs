#!/usr/bin/env node
/**
 * Gate do registro de módulos (SPEC-D-013 `D83` e `D84`).
 *
 * Sem dependências, roda em segundos, sem build. Cada regra é uma afirmação da
 * spec referencial transformada em comando executável — afirmação que não é
 * executada é afirmação que não existe.
 *
 * As regras saem de `spec-referencial-workspace-academico.md`:
 *
 * 1. **As entradas são exatamente as nove de §2 linhas 37-45**, e os utilitários
 *    exatamente os quatro de §2 linhas 51-55. Nenhum módulo a mais, nenhum a menos.
 * 2. **Os nomes são fechados** (§2 linha 58). "Sala de treino" é o nome;
 *    "Laboratório de Testes" foi abandonado e não pode reaparecer.
 * 3. **Teto de quatro destinos por módulo** (§1.9 linha 26, `[D]`). O teto conta
 *    os destinos **declarados** no registro, e §2 linha 72 é o único caso que o
 *    excede: TCC tem a lista `Projetos` na raiz do módulo e quatro dentro do
 *    projeto. A exceção é visível e justificada, não silenciosa.
 * 4. **Nenhum módulo sem dono de dado** — `SPEC-D-007` `D65` exige que a §3.1
 *    seja verificável, e a primeira coisa a verificar é que a lista existe.
 * 5. **Rótulo de módulo não é declarado fora do registro.** Nome de módulo é
 *    decisão de produto; em dois lugares, diverge. A regra é sobre a
 *    **declaração**, não sobre o uso: citar "Faculdade" como `layer.id` é
 *    legítimo e é o que `0002_seed_layers.sql` faz.
 * 6. **Rota de destino começa com a rota do módulo**, e nenhuma rota se repete.
 * 7. **Não há navegação por `useState` de destino** (§1.2 linha 19 e `D84`).
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;
const SRC = join(ROOT, "src");
const SHELL = join(SRC, "shell");

/** Os nove módulos de §2 linhas 37-45, na ordem do documento. */
const MODULOS_ESPERADOS = [
  "Home",
  "Calendário",
  "Faculdade",
  "Estudos",
  "Estágio",
  "TCC & Projetos",
  "Sala de treino",
  "Conhecimento",
  "Biblioteca",
];

/** Os quatro utilitários de §2 linhas 51-55. */
const UTILIDADES_ESPERADOS = ["Ceci", "Busca", "Notificações", "Configurações / Perfil"];

/** §2 linha 58: nome proibido, porque foi abandonado no documento. */
const ROTULOS_PROIBIDOS = ["Laboratório de Testes", "Sala de atendimento", "Treinar"];

/** Teto de §1.9 linha 26, `[D]`. */
const TETO_DESTINOS = 4;

/**
 * Módulos que declaram explicitamente mais de quatro destinos, com o motivo.
 * §2 linha 72 é o único caso: TCC tem a lista `Projetos` na raiz do módulo e
 * quatro destinos dentro do projeto. O gate conta por prefixo de rota, então a
 * exceção é visível e justificada, em vez de silenciosa.
 */
const EXCECOES_TETO = {
  "TCC & Projetos": "§2 linha 72 — lista na raiz do módulo e quatro dentro do projeto",
};

/**
 * Extrai o array `MODULES` e `UTILIDADES` do registro lendo os literais.
 *
 * O gate não importa o módulo: `modules.ts` é TypeScript e este script roda em
 * Node sem build. Ler o texto é o que garante que o gate não dependa de nada que
 * ele deveria estar verificando.
 */
function lerRegistro() {
  const arquivo = join(SHELL, "modules.ts");
  if (!exists(arquivo)) {
    console.error("Fronteiras VIOLADAS:\n  src/shell/modules.ts não existe (D83)");
    process.exit(1);
  }
  const texto = readFileSync(arquivo, "utf8");
  const modulos = bloco(texto, "export const MODULES");
  const utilidades = bloco(texto, "export const UTILIDADES");
  if (!modulos || !utilidades) {
    console.error("Fronteiras VIOLADAS:\n  src/shell/modules.ts não declara MODULES e UTILIDADES");
    process.exit(1);
  }
  return { texto, modulos, utilidades };
}

/** Extrai o corpo do array que começa em `marcador` e termina no `];`. */
function bloco(texto, marcador) {
  const inicio = texto.indexOf(marcador);
  if (inicio === -1) return null;
  const fim = texto.indexOf("];", inicio);
  if (fim === -1) return null;
  return texto.slice(inicio, fim);
}

/** Todos os pares `{ chave: "valor" }` do corpo, na ordem em que aparecem. */
function pares(texto, chave) {
  const re = new RegExp(`${chave}:\\s*"([^"]*)"`, "g");
  const out = [];
  let m;
  while ((m = re.exec(texto)) !== null) out.push(m[1]);
  return out;
}

/**
 * Lê as entradas de módulo do corpo do array.
 *
 * A distinção é de **indentação**: as propriedades de um módulo estão a quatro
 * espaços e as de um destino a oito. Sem isso, ler `rotulo` do corpo inteiro
 * devolve também os rótulos de destino, e a lista de módulos vira uma lista de
 * quarenta e tantos nomes — que é exatamente o tipo de contagem errada que este
 * gate existe para pegar.
 */
function lerEntradas(corpo) {
  const cabecalho = /\n {4}id: "([^"]+)",\n {4}rotulo: "([^"]+)",/g;
  const entradas = [];
  let m;
  while ((m = cabecalho.exec(corpo)) !== null) {
    const inicio = m.index;
    // O bloco do módulo vai até a próxima entrada de módulo ou o fim do array.
    const proximo = cabecalho.lastIndex;
    const fim = corpo.indexOf("\n  {", proximo);
    const bloco = corpo.slice(inicio, fim === -1 ? corpo.length : fim);
    const dono = bloco.match(/dono: \[([^\]]*)\]/s);
    // Contar por `rota:` e não por indentação: `prettier` decide se o objeto de um
    // destino cabe em uma linha, e uma contagem que depende de isso erra sem
    // aviso. Um destino tem uma rota, e o módulo tem a raiz — daí o `- 1`.
    const nRotas = (bloco.match(/\brota:/g) ?? []).length;
    entradas.push({
      id: m[1],
      rotulo: m[2],
      dono: dono ? (dono[1].match(/"[^"]+"/g) ?? []).map((s) => s.slice(1, -1)) : [],
      rota: (bloco.match(/\n {4}rota: "([^"]+)"/) ?? [])[1] ?? "",
      nDestinos: Math.max(0, nRotas - 1),
      rotas: [...bloco.matchAll(/rota: "([^"]+)"/g)].map((x) => x[1]).slice(1),
    });
  }
  return entradas;
}

/** Um segmento de rota, com o prefixo de módulo. */
function segmentoDeRota(rota, raizDoModulo) {
  if (!rota.startsWith(raizDoModulo)) return rota;
  const resto = rota.slice(raizDoModulo.length);
  if (resto === "") return "";
  return resto.split("/")[1] ?? "";
}

function exists(p) {
  try {
    statSync(p);
    return true;
  } catch {
    return false;
  }
}

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "dist" || entry.startsWith(".")) continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (/\.(ts|tsx)$/.test(full)) out.push(full);
  }
  return out;
}

function stripComments(text) {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "))
    .replace(/(^|[^:])\/\/[^\n]*/g, (_m, prefix) => prefix);
}

const failures = [];
const rel = (f) => relative(ROOT, f);

// ---------------------------------------------------------------- registro

const { texto: registroTexto, modulos: blocoModulos, utilidades: blocoUtilidades } = lerRegistro();

const entradas = lerEntradas(blocoModulos);
const utilitarios = lerEntradas(blocoUtilidades);

const rotulosModulos = entradas.map((e) => e.rotulo);
const rotulosUtilidades = utilitarios.map((e) => e.rotulo);
const todosOsRotulos = [...rotulosModulos, ...rotulosUtilidades];
const rotas = [...entradas.map((e) => e.rota), ...utilitarios.map((e) => e.rota)];

// Regra 1 — as entradas são as nove e os quatro.
if (rotulosModulos.join("|") !== MODULOS_ESPERADOS.join("|")) {
  failures.push(
    `  src/shell/modules.ts  entradas do Workspace Acadêmico não são as nove de §2 linhas 37-45\n` +
      `      esperado: ${MODULOS_ESPERADOS.join(" · ")}\n` +
      `      obtido:   ${rotulosModulos.join(" · ")}`,
  );
}
if (rotulosUtilidades.join("|") !== UTILIDADES_ESPERADOS.join("|")) {
  failures.push(
    `  src/shell/modules.ts  utilitários não são os quatro de §2 linhas 51-55\n` +
      `      esperado: ${UTILIDADES_ESPERADOS.join(" · ")}\n` +
      `      obtido:   ${rotulosUtilidades.join(" · ")}`,
  );
}

// Regra 2 — nomes fechados.
for (const proibido of ROTULOS_PROIBIDOS) {
  for (const rotulo of [...rotulosModulos, ...rotulosUtilidades]) {
    if (rotulo === proibido) {
      failures.push(
        `  src/shell/modules.ts  "${proibido}" foi abandonado por §2 linha 58 e não pode ser nome de módulo`,
      );
    }
  }
}

// Regra 3 — teto de destinos declarados.
for (const entrada of entradas) {
  const n = entrada.nDestinos;
  if (n > TETO_DESTINOS && !EXCECOES_TETO[entrada.rotulo]) {
    failures.push(
      `  src/shell/modules.ts  "${entrada.rotulo}" declara ${n} destinos e o teto de §1.9 linha 26 é ${TETO_DESTINOS}`,
    );
  }
  if (n === 0) {
    failures.push(
      `  src/shell/modules.ts  "${entrada.rotulo}" não declara nenhum destino de navegação`,
    );
  }
}

// Regra 4 — todo módulo tem dono de dado declarado, e a rota de destino começa
// com a rota do módulo.
for (const entrada of entradas) {
  if (entrada.dono.length === 0) {
    failures.push(
      `  src/shell/modules.ts  "${entrada.rotulo}" não declara dono de dado; §3.1 exige um`,
    );
  }
  for (const rota of entrada.rotas) {
    if (rota !== entrada.rota && !rota.startsWith(`${entrada.rota}/`)) {
      failures.push(
        `  src/shell/modules.ts  rota de destino "${rota}" não começa com a rota do módulo "${entrada.rotulo}" (${entrada.rota})`,
      );
    }
  }
}

// Regra 6 — nenhuma rota se repete.
const rotaVistas = new Set();
for (const rota of rotas) {
  if (rotaVistas.has(rota)) {
    failures.push(`  src/shell/modules.ts  rota repetida: ${rota}`);
  }
  rotaVistas.add(rota);
  if (rota !== "/" && !rota.startsWith("/")) {
    failures.push(`  src/shell/modules.ts  rota sem barra inicial: ${rota}`);
  }
}

// ------------------------------------------------- declaração fora do registro

const FORA_DO_REGISTRO = [join(SHELL, "modules.ts"), join(SHELL, "__tests__", "modules.test.ts")];
const arquivos = walk(SRC).filter((f) => !FORA_DO_REGISTRO.includes(f));

for (const arquivo of arquivos) {
  const codigo = stripComments(readFileSync(arquivo, "utf8"));
  codigo.split("\n").forEach((linha, i) => {
    for (const rotulo of todosOsRotulos) {
      // Regra 5 — a **declaração** de um rótulo de módulo é do registro. Citar o
      // nome em outro lugar é legítimo: `layers.ts` declara `layer.id =
      // "faculdade"`, e isso é dado, não catálogo de módulo.
      if (new RegExp(`\\brotulo:\\s*["']${rotulo}["']`).test(linha)) {
        failures.push(
          `  ${rel(arquivo)}:${i + 1}  rótulo "${rotulo}" só pode ser declarado em src/shell/modules.ts (D83)`,
        );
      }
    }
    // Regra 7 — nada de navegação por estado.
    if (/useState<\s*View\s*>/.test(linha)) {
      failures.push(
        `  ${rel(arquivo)}:${i + 1}  useState<View> é navegação por estado; D84 exige rota`,
      );
    }
  });
}

// ------------------------------------------------------------------ saída

const destinosCount = entradas.reduce((n, e) => n + e.nDestinos, 0);

if (failures.length > 0) {
  console.error(`Registro de módulos VIOLADO (${failures.length}):\n`);
  console.error(failures.join("\n"));
  console.error(
    "\nVer src/shell/modules.ts e SPEC-D-013. A lista é §2 linhas 37-45 e 51-55 do referencial.",
  );
  process.exit(1);
}

console.log(
  `OK: registro fechado (${entradas.length} módulos + ${utilitarios.length} utilitários, ` +
    `${destinosCount} destinos de navegação, ${arquivos.length} arquivos verificados).`,
);
