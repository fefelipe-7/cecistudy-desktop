/**
 * cecistudy — verificador do contrato de banco
 *
 * Executa `schema.sql` em SQLite em memória e confere:
 *   1. O DDL completo roda sem erro (IF NOT EXISTS — idempotente).
 *   2. As tabelas dos DOIS bancos lógicos existem (cecistudy_user + catalog).
 *   3. As versões no header do .sql batem com as do app que declara o payload.
 *
 * O passo 3 é o que pega o débito C2: `SCHEMA_VERSION` e `USER_SCHEMA_VERSION`
 * têm que ter **um** valor. Cada app declara o seu em TypeScript, e aqui os dois
 * são comparados com o header do contrato.
 *
 * Uso:
 *   node contratos/dados/verify-schema.mjs
 *   node contratos/dados/verify-schema.mjs --schema-ts=<caminho>/schema.ts
 *   CECISTUDY_SCHEMA_TS=<caminho> node contratos/dados/verify-schema.mjs
 *
 * Sem `--schema-ts` o script procura `packages/data/src/schema.ts` subindo a
 * partir do próprio diretório, o que funciona no layout local do grupo. Em CI,
 * onde os dois repositórios são irmãos, o caminho tem que vir explícito.
 *
 * Em falha → exit 1. Em sucesso → exit 0.
 */
import { readFileSync, existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const argv = process.argv.slice(2);
const flag = (nome) =>
  argv
    .find((a) => a.startsWith(`--${nome}=`))
    ?.split("=")
    .slice(1)
    .join("=");

/** Procura `packages/data/src/schema.ts` subindo a partir de `de`. */
function achaSchemaTs(de) {
  let dir = de;
  for (let i = 0; i < 8; i += 1) {
    const candidato = resolve(dir, "packages/data/src/schema.ts");
    if (existsSync(candidato)) return candidato;
    const pai = dirname(dir);
    if (pai === dir) break;
    dir = pai;
  }
  return null;
}

const schemaTsPath = flag("schema-ts") ?? process.env.CECISTUDY_SCHEMA_TS ?? achaSchemaTs(HERE);

// node:sqlite (Node 22+ experimental) — usa só se disponível; senão o Node sqlite3.
const { DatabaseSync } = await import("node:sqlite");

const sql = readFileSync(resolve(HERE, "schema.sql"), "utf8");

// ---- 3. Versões no header do contrato vs o app que declara o payload ----
const header = (nome) => sql.match(new RegExp(`${nome}\\s*=\\s*(\\d+)`))?.[1];

if (!schemaTsPath) {
  console.error(
    "✗ não achei packages/data/src/schema.ts.\n" +
      "  Passe --schema-ts=<caminho> ou CECISTUDY_SCHEMA_TS=<caminho>.\n" +
      "  Sem esse passo o verificador não cobre o débito C2 (SCHEMA_VERSION e\n" +
      "  USER_SCHEMA_VERSION têm que ter um valor só).",
  );
  process.exit(1);
}

const schemaTs = readFileSync(schemaTsPath, "utf8");
const noTs = (nome) => schemaTs.match(new RegExp(`${nome}\\s*=\\s*(\\d+)`))?.[1];

const divergencias = [];
for (const nome of ["SCHEMA_VERSION", "USER_SCHEMA_VERSION"]) {
  const contrato = header(nome);
  const app = noTs(nome);
  if (contrato === undefined) {
    divergencias.push(`${nome}: ausente no header de schema.sql`);
  } else if (app === undefined) {
    divergencias.push(`${nome}: ausente em ${schemaTsPath}`);
  } else if (contrato !== app) {
    divergencias.push(`${nome}: schema.sql="${contrato}" vs app="${app}"`);
  }
}

// ---- 3b. Débito C2: um valor só para USER_SCHEMA_VERSION ----
// A constante aparece em dois arquivos do mobile e eles não podem discordar:
// `packages/data/src/schema.ts` (payload) e `src/lib/db/migrations/user.ts`
// (base SQLite). Enquanto houver dois, um app migra e o outro não.
const userTsPath = resolve(dirname(schemaTsPath), "../../../src/lib/db/migrations/user.ts");
if (existsSync(userTsPath)) {
  const userTs = readFileSync(userTsPath, "utf8");
  const noUserTs = userTs.match(/USER_SCHEMA_VERSION\s*=\s*(\d+)/)?.[1];
  const noSchemaTs = noTs("USER_SCHEMA_VERSION");
  if (noUserTs !== undefined && noSchemaTs !== undefined && noUserTs !== noSchemaTs) {
    divergencias.push(
      `USER_SCHEMA_VERSION: ${userTsPath}="${noUserTs}" vs ` +
        `${schemaTsPath}="${noSchemaTs}" (débito C2)`,
    );
  }
}

if (divergencias.length) {
  console.error(`✗ versão divergente:\n  ${divergencias.join("\n  ")}`);
  process.exit(1);
}

// ---- 1. Aplica o DDL num banco único em memória ----
// (O .sql declara DOIS bancos lógicos num só arquivo; em memória rodamos tudo
// no mesmo arquivo — o objetivo aqui é validar sintaxe DDL e completude.)
let db;
try {
  db = new DatabaseSync(":memory:");
  db.exec(sql);
} catch (err) {
  console.error("✗ DDL exec falhou:", err.message);
  process.exit(1);
}

// ---- 2. Completude de tabelas por banco lógico ----
const allTables = new Set(
  db
    .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
    .all()
    .map((r) => r.name),
);

/** Tabelas esperadas do banco da usuária (spelhadas de `src/lib/db/migrations/user.ts`). */
const USER_TABLES = [
  "profile",
  "course",
  "course_schedule",
  "class_note",
  "class_note_link",
  "task",
  "task_link",
  "assessment",
  "assessment_topic",
  "study_session",
  "flashcard",
  "reading",
  "reading_highlight",
  "reading_progress",
  "author",
  "concept",
  "concept_course",
  "concept_author",
  "material",
  "technique",
  "note",
  "note_link",
  "internship",
  "internship_concept",
  "internship_topic",
  "supervision_notebook",
  "thesis_project",
  "thesis_chapter",
  "thesis_reference",
  "achievement",
  "quiz_session",
  "quiz_answer",
  "saved_catalog_item",
  "streak",
  "activity_event",
  "legacy_import_map",
];

/** Tabelas esperadas do catálogo (espelhadas de `src/lib/db/catalogSchema.ts`). */
const CATALOG_TABLES = [
  "catalog_release",
  "area",
  "approach_family",
  "approach",
  "question",
  "work",
  "concept_domain",
  "concept",
  "catalog_author",
  "technique_category",
  "technique",
  "comparison",
  "question_category",
  "topic",
];

const missing = [...USER_TABLES, ...CATALOG_TABLES].filter((t) => !allTables.has(t));
if (missing.length) {
  console.error(`✗ tabelas ausentes: ${missing.join(", ")}`);
  process.exit(1);
}

console.log(
  `✓ schema.sql válido (SCHEMA_VERSION=${header("SCHEMA_VERSION")}, ` +
    `USER_SCHEMA_VERSION=${header("USER_SCHEMA_VERSION")}); ` +
    `${allTables.size} tabelas criadas.`,
);
db.close();
