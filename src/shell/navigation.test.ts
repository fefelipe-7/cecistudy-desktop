import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  CANDIDATOS_RECUSADOS,
  MODULOS,
  UTILITARIOS,
  WORKSPACES,
  moduloPorId,
  type ModuleId,
} from "./navigation.ts";

/**
 * A ordem e os rótulos são o contrato de produto de `SPEC-D-006` D63: a sidebar
 * lê esta tabela, então um rótulo que muda aqui é a falha de aceite, não um
 * ajuste de implementação.
 */
const ORDEM_ESPERADA: readonly { id: ModuleId; titulo: string }[] = [
  { id: "home", titulo: "Home" },
  { id: "calendario", titulo: "Calendário" },
  { id: "faculdade", titulo: "Faculdade" },
  { id: "estudos", titulo: "Estudos" },
  { id: "biblioteca", titulo: "Biblioteca" },
  { id: "conhecimento", titulo: "Conhecimento" },
  { id: "tcc", titulo: "TCC & Projetos" },
  { id: "estagio", titulo: "Estágio" },
  { id: "sala-de-treino", titulo: "Sala de treino" },
];

describe("MODULOS", () => {
  it("tem nove entradas, uma por ModuleId, sem repetir id", () => {
    assert.equal(MODULOS.length, 9);
    assert.equal(new Set(MODULOS.map((m) => m.id)).size, MODULOS.length);
  });

  it("a ordem da sidebar é a de §2 e o rótulo é o nome fechado", () => {
    assert.deepEqual(
      MODULOS.map((m) => ({ id: m.id, titulo: m.titulo })),
      ORDEM_ESPERADA,
    );
  });

  // O `deepEqual` de ordem e rótulos acima já é a garantia de I4: nenhum título
  // pode ser um nome abandonado, porque todo título é exatamente o nome fechado.

  it("Sala de treino é o rótulo do módulo de paciente simulado", () => {
    const sala = MODULOS.find((m) => m.id === "sala-de-treino");
    assert.equal(sala?.titulo, "Sala de treino");
  });

  it("toda entrada declara dono, e a Home é a única sem dono", () => {
    for (const modulo of MODULOS) {
      if (modulo.id === "home") {
        assert.equal(modulo.dono, null, "home não é dona de nada (§4.1)");
      } else {
        assert.equal(modulo.dono, modulo.id, `${modulo.id} deve declarar o próprio dono`);
      }
    }
  });

  it("todo módulo tem descrição não vazia e estado declarado", () => {
    for (const modulo of MODULOS) {
      assert.ok(modulo.descricao.length > 0, `${modulo.id} sem descrição`);
      assert.ok(
        modulo.estado === "pronto" || modulo.estado === "em-construcao",
        `${modulo.id} com estado inválido`,
      );
    }
  });

  it("os módulos implementados estão prontos; os demais existem com estado explícito", () => {
    const prontos = MODULOS.filter((m) => m.estado === "pronto").map((m) => m.id);
    assert.deepEqual(prontos, ["calendario", "faculdade", "estudos", "tcc"]);
  });
});

describe("moduloPorId", () => {
  it("devolve a entrada e é total sobre a união de ids", () => {
    for (const { id } of ORDEM_ESPERADA) {
      assert.equal(moduloPorId(id).id, id);
    }
  });
});

describe("UTILITARIOS", () => {
  it("são os três do rodapé, sem o nome antigo da IA (SPEC-D-014 D94)", () => {
    assert.deepEqual([...UTILITARIOS], ["cecinho", "busca", "notificacoes"]);
    for (const id of UTILITARIOS) {
      assert.ok(!MODULOS.some((m) => (m.id as string) === id));
    }
  });
});

describe("WORKSPACES", () => {
  it("o Acadêmico é pronto e o Profissional existe com estado explícito", () => {
    const academico = WORKSPACES.find((w) => w.id === "academico");
    const profissional = WORKSPACES.find((w) => w.id === "profissional");
    assert.equal(academico?.estado, "pronto");
    assert.equal(profissional?.estado, "em-construcao");
  });
});

describe("CANDIDATOS_RECUSADOS", () => {
  it("cada candidato declara contidoEm, dono e motivo", () => {
    for (const candidato of CANDIDATOS_RECUSADOS) {
      assert.ok(candidato.titulo.length > 0);
      assert.ok(candidato.motivo.length > 0, `${candidato.titulo} sem motivo`);
      assert.ok(moduloPorId(candidato.contidoEm).id === candidato.contidoEm);
      assert.ok(moduloPorId(candidato.dono).id === candidato.dono);
    }
  });

  it("nomeia ao menos as entidades óbvias que §1.2 recusa", () => {
    const titulos = CANDIDATOS_RECUSADOS.map((c) => c.titulo);
    for (const esperado of [
      "Evento",
      "Responsabilidade",
      "Regra de recorrência",
      "Capítulo",
      "Flashcard avulso",
    ]) {
      assert.ok(titulos.includes(esperado), `falta ${esperado}`);
    }
  });
});
