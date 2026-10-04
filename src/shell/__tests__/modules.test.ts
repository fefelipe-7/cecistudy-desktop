import { test } from "node:test";
import assert from "node:assert/strict";

import {
  gruposDaSidebar,
  MODULES,
  ORDEM_SIDEBAR,
  SECOES_SIDEBAR,
  UTILIDADES,
  moduloDaRota,
  moduloPorId,
  totalDeDestinos,
  todosOsDestinos,
} from "../modules.ts";

/**
 * Testes do registro de módulos — `SPEC-D-013` `D83`, invariantes `I1` e `I2`.
 *
 * `scripts/check-modules.mjs` roda as mesmas regras em cima do **texto** do
 * registro, para não depender de bundler. Aqui elas rodam em cima do **valor**, que
 * é o que pega erro de digitação em literal de tipo. Os dois são necessários: o
 * gate lê o que está escrito, o teste lê o que está compilado.
 */

const MODULOS_DE_REFERENCIAL = [
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

const UTILIDADES_DE_REFERENCIAL = ["Ceci", "Busca", "Notificações", "Configurações / Perfil"];

test("I2 · as entradas são as nove de §2 linhas 37-45, na ordem do documento", () => {
  assert.deepEqual(
    MODULES.map((m) => m.rotulo),
    MODULOS_DE_REFERENCIAL,
  );
});

test("I2 · os utilitários são os quatro de §2 linhas 51-55", () => {
  assert.deepEqual(
    UTILIDADES.map((u) => u.rotulo),
    UTILIDADES_DE_REFERENCIAL,
  );
});

test("§2 linha 58 · nenhum nome abandonado reaparece", () => {
  const rotulos = [...MODULES, ...UTILIDADES].map((e) => e.rotulo);
  for (const proibido of ["Laboratório de Testes", "Sala de atendimento", "Treinar"]) {
    assert.ok(!rotulos.includes(proibido), `nome abandonado presente: ${proibido}`);
  }
});

test("§1.9 linha 26 · teto de quatro destinos, exceto TCC por §2 linha 72", () => {
  for (const modulo of MODULES) {
    if (modulo.rotulo === "TCC & Projetos") {
      assert.equal(modulo.destinos.length, 5);
      continue;
    }
    assert.ok(
      modulo.destinos.length <= 4,
      `${modulo.rotulo} tem ${modulo.destinos.length} destinos e o teto é 4`,
    );
  }
});

test("§4.1 linha 120 · Home não é dona de nada", () => {
  assert.deepEqual(moduloPorId("home").dono, ["nenhum"]);
});

test("§3.1 · todo módulo declara dono de dado", () => {
  for (const modulo of MODULES) {
    assert.ok(modulo.dono.length > 0, `${modulo.rotulo} sem dono declarado`);
  }
});

test("a soma dos destinos de §2 linhas 64-74 é 31", () => {
  // 1 Home + 4 Calendário + 4 Faculdade + 4 Estudos + 1 Biblioteca
  // + 4 Conhecimento + 5 TCC + 4 Estágio + 4 Sala de treino = 31.
  assert.equal(totalDeDestinos(), 31);
});

test("toda rota de destino começa com a rota do seu módulo", () => {
  for (const modulo of MODULES) {
    for (const destino of modulo.destinos) {
      assert.ok(
        destino.rota === modulo.rota || destino.rota.startsWith(`${modulo.rota}/`),
        `${destino.rota} não começa com ${modulo.rota}`,
      );
    }
  }
});

test("nenhuma rota se repete", () => {
  const rotas = todosOsDestinos().map((d) => d.rota);
  assert.equal(new Set(rotas).size, rotas.length);
});

test("§2 linha 60 · a sidebar é uma ordem, e ela cobre os nove módulos", () => {
  assert.equal(ORDEM_SIDEBAR.length, MODULES.length);
  assert.deepEqual([...ORDEM_SIDEBAR].sort(), MODULES.map((m) => m.id).sort());
});

test("a ordem da sidebar difere da árvore em três pares, como §2 linha 60 descreve", () => {
  // A árvore (§2 linhas 37-45) põe Estágio e TCC antes de Conhecimento e
  // Biblioteca; a sidebar põe Biblioteca e Conhecimento primeiro, e TCC antes de
  // Estágio. Se as duas algum dia coincidirem, a linha 60 deixou de descrever o
  // que o código faz, e é a linha 60 que está errada.
  assert.ok(ORDEM_SIDEBAR.indexOf("biblioteca") < ORDEM_SIDEBAR.indexOf("tcc"));
  assert.ok(ORDEM_SIDEBAR.indexOf("conhecimento") < ORDEM_SIDEBAR.indexOf("tcc"));
  assert.ok(ORDEM_SIDEBAR.indexOf("tcc") < ORDEM_SIDEBAR.indexOf("estagio"));
});

test("as seções cobrem os nove módulos, uma vez cada, sem repetir nem omitir", () => {
  // Agrupar é decisão de desenho, mas **cobrir tudo** é uma invariante: um módulo
  // que some de `SECOES_SIDEBAR` fica inalcançável, e o sintoma é uma sidebar mais
  // bonita com uma entrada a menos. O gate é este teste, não a lista.
  const nasSecoes = SECOES_SIDEBAR.flatMap((secao) => secao.modulos);
  assert.equal(nasSecoes.length, new Set(nasSecoes).size, "um módulo em duas seções");
  assert.deepEqual([...nasSecoes].sort(), [...ORDEM_SIDEBAR].sort());
});

test("a ordem dentro das seções respeita ORDEM_SIDEBAR, que é §2 linha 60", () => {
  // §2 linha 60 é `[P]`, e a ordem é o dado. Se as seções reordenassem por conta
  // própria, a linha 60 deixaria de descrever o que o código faz.
  const posicao = new Map(ORDEM_SIDEBAR.map((id, i) => [id, i]));
  for (const secao of SECOES_SIDEBAR) {
    const indices = secao.modulos.map((id) => posicao.get(id) ?? -1);
    assert.deepEqual(
      indices,
      [...indices].sort((a, b) => a - b),
      `seção "${secao.nome}" fora de ordem`,
    );
  }
});

test("nenhuma seção repete nome, e nenhuma seção fica vazia", () => {
  const nomes = SECOES_SIDEBAR.map((s) => s.nome);
  assert.equal(nomes.length, new Set(nomes).size);
  for (const secao of SECOES_SIDEBAR) {
    assert.ok(secao.nome.trim().length > 0, `seção ${secao.id} sem nome`);
    assert.ok(secao.modulos.length > 0, `seção ${secao.nome} vazia`);
  }
});

test("gruposDaSidebar devolve entradas do registro, e nunca um id solto", () => {
  // D83: o rótulo do módulo tem um dono só. Se `gruposDaSidebar` devolvesse
  // `ModuleId`, quem desenha a sidebar seria obrigado a resolver o nome — e aí
  // nasceria uma segunda lista de rótulos, que é o defeito que o gate reprova.
  for (const secao of gruposDaSidebar()) {
    for (const modulo of secao.modulos) {
      assert.equal(modulo, moduloPorId(modulo.id));
      assert.ok(MODULES.includes(modulo));
      assert.ok(modulo.rotulo.length > 0);
    }
  }
});

test("toda entrada com bloqueio declara o motivo, e nenhum bloqueio está vazio", () => {
  for (const modulo of MODULES) {
    if (!modulo.bloqueadoPor) continue;
    assert.ok(modulo.bloqueadoPor.length > 0);
    for (const motivo of modulo.bloqueadoPor) assert.match(motivo, /\[A\]/);
  }
  for (const modulo of MODULES) {
    for (const destino of modulo.destinos) {
      if (!destino.bloqueadoPor) continue;
      assert.ok(destino.bloqueadoPor.length > 0);
      for (const motivo of destino.bloqueadoPor) assert.match(motivo, /\[A\]/);
    }
  }
});

test("Configurações não tem posição na sidebar, e §2 linha 60 é quem omite", () => {
  // §2 linha 60 lista o rodapé com três itens e não menciona Configurações. O
  // registro diz `fora` em vez de inventar um lugar.
  const configuracoes = UTILIDADES.find((u) => u.id === "configuracoes");
  assert.ok(configuracoes);
  assert.equal(configuracoes.posicao, "fora");
});

test("moduloDaRota resolve o módulo de uma rota de destino e de uma de utilitário", () => {
  assert.equal(moduloDaRota("/calendario/semana")?.id, "calendario");
  assert.equal(moduloDaRota("/calendario")?.id, "calendario");
  assert.equal(moduloDaRota("/tcc/projetos/abc/escrita")?.id, "tcc");
  assert.equal(moduloDaRota("/configuracoes"), undefined);
});

test("toda entrada declara a spec que a implementa", () => {
  // Nenhuma entrada do registro pode dizer `null`: ou tem spec, ou a spec ainda
  // não foi escrita — e "ainda não foi escrita" é informação que a tela de
  // destino mostra.
  for (const modulo of MODULES) {
    for (const destino of modulo.destinos) {
      assert.ok(destino.spec, `${modulo.rotulo}/${destino.rotulo} sem SPEC-D declarada`);
      assert.match(destino.spec, /^SPEC-D-\d{3}$/);
    }
  }
  for (const utilidade of UTILIDADES) {
    assert.ok(utilidade.spec);
    assert.match(utilidade.spec, /^SPEC-D-\d{3}$/);
  }
});
