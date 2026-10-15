#!/usr/bin/env node
/**
 * Verificação negativa do gate de fronteiras.
 *
 * Um gate que nunca falhou não é evidência de nada. Este script monta uma
 * cópia do que o gate lê, injeta violações conhecidas e exige que ele acuse.
 * Se um dia o gate deixar de ver uma delas, isto falha — e é para isso que ele
 * existe. Foi o mesmo raciocínio de F8b2, aplicado ao desenho da regra 8: ela
 * lia `src-tauri/src/commands.rs` por nome, e dividir aquele arquivo a deixava
 * verde sobre uma violação real de D67.
 *
 * Os casos cobrem `pub fn` e `fn` privado de propósito. A regra 8 precisa
 * analisar os dois: o núcleo privado é alcançado pela casca `pub` do mesmo
 * módulo, então ele também é caminho de escrita. Testar só um dos dois devolve
 * metade da regra.
 *
 * Uso: `npm run test:gate`
 */

import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const RAIZ = fileURLToPath(new URL("..", import.meta.url));

/** Copia só o que o gate lê: `src/`, as migrations e o código Rust. */
function montarFixture() {
  const dir = mkdtempSync(join(tmpdir(), "gate-negativo-"));
  cpSync(join(RAIZ, "scripts"), join(dir, "scripts"), { recursive: true });
  cpSync(join(RAIZ, "src"), join(dir, "src"), { recursive: true });
  cpSync(join(RAIZ, "src-tauri", "migrations"), join(dir, "src-tauri", "migrations"), {
    recursive: true,
  });
  cpSync(join(RAIZ, "src-tauri", "src"), join(dir, "src-tauri", "src"), { recursive: true });
  return dir;
}

function rodarGate(dir) {
  try {
    const saida = execFileSync(process.execPath, [join(dir, "scripts", "check-boundaries.mjs")], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { codigo: 0, saida };
  } catch (erro) {
    return { codigo: erro.status ?? 1, saida: `${erro.stdout ?? ""}${erro.stderr ?? ""}` };
  }
}

/**
 * Insere um trecho no corpo de produção de um arquivo de escrita.
 *
 * Se ainda houver `#[cfg(test)] mod … {` no arquivo, insere antes dele — o
 * gate corta ali e não contaria escrita de teste. Depois da migração (F1..F5) o
 * teste vive em `<módulo>/testes.rs` e o arquivo de escrita não tem mais corte,
 * então a inserção vai ao fim, que ainda é produção.
 */
function inserirNoCorpo(fonte, trecho) {
  const linhas = fonte.split("\n");
  for (let i = 0; i < linhas.length; i++) {
    if (linhas[i].startsWith("#[cfg(test)]") && /^mod\s+\w+\s*\{/.test(linhas[i + 1] ?? "")) {
      linhas.splice(i, 0, trecho);
      return linhas.join("\n");
    }
  }
  return `${fonte}\n${trecho}`;
}

/**
 * Uma escrita que viola D67: o comando é invocado pela bridge de
 * `faculdade` e grava `responsibility`, que é do `calendario`.
 */
function escritaQueViola(visibilidade) {
  const assinatura = visibilidade === "privado" ? "fn injetada_core" : "pub fn injetada_core";
  return `
// ===== INJEÇÃO DE PROVA — ${visibilidade} =====
${assinatura}(db: &Db) -> Result<(), DbError> {
  db.write(|tx| {
    tx.execute("UPDATE responsibility SET title = ?1 WHERE id = ?2", params!["x", "y"])?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn injetada(db: tauri::State<'_, Db>) -> Result<(), DbError> {
  injetada_core(&db)
}
// ===== FIM DA INJEÇÃO =====
`;
}

/**
 * AViolation vai para o caminho de escrita de um módulo de verdade: a regra 8
 * tira o dono do **diretório**, então é lá que a violação precisa estar para
 * provar que a regra o vê.
 */
const COMANDOS_DE = ["calendario", "tcc", "estudos", "faculdade"];

function arquivoDeComandos(dir) {
  for (const modulo of COMANDOS_DE) {
    const p = join(dir, "src-tauri", "src", modulo, "comandos.rs");
    if (existsSync(p)) return p;
  }
  throw new Error("nenhum <módulo>/comandos.rs no fixture — a migração quebrou?");
}

/** Registra a injeção na bridge de `faculdade`, para o cruzamento de D67. */
function registrarNaBridgeDeFaculdade(dir) {
  const b = join(dir, "src", "features", "faculdade", "data", "bridge.ts");
  writeFileSync(
    b,
    readFileSync(b, "utf8") +
      '\n\nexport async function injetada(): Promise<void> {\n  await invoke<void>("injetada");\n}\n',
  );
}

const casos = [
  {
    nome: "núcleo público gravando tabela de outro dono",
    preparar(dir) {
      const p = arquivoDeComandos(dir);
      writeFileSync(p, inserirNoCorpo(readFileSync(p, "utf8"), escritaQueViola("publico")));
      registrarNaBridgeDeFaculdade(dir);
    },
    esperado: /`injetada` escreve em tabela de `calendario`/,
  },
  {
    nome: "núcleo privado gravando tabela de outro dono",
    preparar(dir) {
      const p = arquivoDeComandos(dir);
      writeFileSync(p, inserirNoCorpo(readFileSync(p, "utf8"), escritaQueViola("privado")));
      registrarNaBridgeDeFaculdade(dir);
    },
    esperado: /`injetada` escreve em tabela de `calendario`/,
  },
  {
    nome: "módulo gravando tabela que não é dele",
    preparar(dir) {
      // `faculdade/comandos.rs` grava `responsibility`, que é do Calendário.
      const mod = join(dir, "src-tauri", "src", "faculdade");
      writeFileSync(
        join(mod, "comandos.rs"),
        `use rusqlite::params;\nuse crate::nucleo::{Db, DbError};\n\n` + escritaQueViola("publico"),
      );
      registrarNaBridgeDeFaculdade(dir);
    },
    esperado: /mora em `faculdade\/` e escreve em tabela de `calendario`/,
  },
  {
    nome: "arquivo da raiz do crate escrevendo tabela de módulo",
    preparar(dir) {
      const p = join(dir, "src-tauri", "src", "perdido.rs");
      writeFileSync(
        p,
        `use rusqlite::params;\nuse crate::nucleo::{Db, DbError};\n\n` + escritaQueViola("publico"),
      );
      registrarNaBridgeDeFaculdade(dir);
    },
    esperado: /`perdido\.rs` está na raiz do crate e escreve em tabela de/,
  },
  {
    nome: "casca registrada que não existe",
    preparar(dir) {
      const p = join(dir, "src-tauri", "src", "lib.rs");
      const linhas = readFileSync(p, "utf8").split("\n");
      // Insere depois da primeira linha de registro que existir, para o caso
      // não depender do formato (`commands::x` ou `mod::comandos::x`).
      const i = linhas.findIndex((l) => /^\s*(?:\w+::)+\w+,?\s*$/.test(l));
      if (i === -1) throw new Error("lib.rs não tem nenhum comando registrado");
      linhas.splice(i + 1, 0, "      nucleo::comandos::nao_existe,");
      writeFileSync(p, linhas.join("\n"));
    },
    esperado: /lib\.rs registra `nao_existe`/,
  },
  {
    nome: "diretório de módulo sem dono declarado",
    preparar(dir) {
      const novo = join(dir, "src-tauri", "src", "biblioteca");
      cpSync(join(dir, "src-tauri", "src", "tcc", "mod.rs"), join(novo, "mod.rs"));
    },
    esperado: /não é um módulo dono declarado/,
  },
];

let falhas = 0;

// 0 — o fixture tem de montar um gate verde. Sem esta linha, um gate quebrado
// "acusaria" toda injeção por acidente e a verificação não provaria nada.
const base = montarFixture();
const inicial = rodarGate(base);
if (inicial.codigo !== 0) {
  console.error("✗ o fixture não parte de um gate verde — a verificação não vale nada:\n");
  console.error(inicial.saida);
  falhas++;
} else {
  console.log(
    `✓ fixture parte de um gate verde${/dívida D98/.test(inicial.saida) ? ", com a dívida nomeada" : ""}`,
  );
}
rmSync(base, { recursive: true, force: true });

for (const caso of casos) {
  const dir = montarFixture();
  try {
    caso.preparar(dir);
    const r = rodarGate(dir);
    if (r.codigo === 0) {
      console.error(`✗ NÃO ACUSOU: ${caso.nome}`);
      console.error("  o gate saiu com 0 numa violação conhecida (D67/F8b2 de novo)\n");
      falhas++;
    } else if (!caso.esperado.test(r.saida)) {
      console.error(`✗ ACUSOU, MAS PELO MOTIVO ERRADO: ${caso.nome}`);
      console.error(`  esperava ${caso.esperado}\n  recebeu:\n${r.saida}\n`);
      falhas++;
    } else {
      console.log(`✓ acusa: ${caso.nome}`);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

if (falhas > 0) {
  console.error(`\n✗ ${falhas} caso(s) de verificação negativa falharam.`);
  process.exit(1);
}
console.log(`\n✓ verificação negativa: ${casos.length} violações injetadas, todas acusadas.`);
