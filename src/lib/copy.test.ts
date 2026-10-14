import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { IDENTIDADE, VAZIO } from "./copy.ts";

/** Coleta recursiva de todo texto exportado, para as regras negativas de D4. */
function textos(valor: unknown, saida: string[] = []): string[] {
  if (typeof valor === "string") saida.push(valor);
  else if (typeof valor === "function") saida.push((valor as (t: string) => string)("exemplo"));
  else if (valor && typeof valor === "object") {
    for (const v of Object.values(valor)) textos(v, saida);
  }
  return saida;
}

describe("IDENTIDADE", () => {
  it("a marca visível é cecistudy ♡ e a técnica é cecistudy (ADR-011)", () => {
    assert.equal(IDENTIDADE.marca, "cecistudy ♡");
    assert.equal(IDENTIDADE.marcaTecnica, "cecistudy");
  });

  it("o nome técnico não contém o glyph ♡ (I1)", () => {
    assert.ok(!IDENTIDADE.marcaTecnica.includes("♡"));
  });

  it("a assistente se chama cecinho (D2/D94)", () => {
    assert.equal(IDENTIDADE.assistente, "cecinho");
  });
});

describe("voz do desktop (D4)", () => {
  it("nenhum texto usa a metáfora cozy nem emoji decorativo", () => {
    const proibidos = ["cantinho", "🧸", "✨", "🎉", "🔥", "☕", "🌱"];
    for (const texto of textos(VAZIO)) {
      for (const proibido of proibidos) {
        assert.ok(
          !texto.includes(proibido),
          `texto com termo fora do escopo do desktop: ${proibido} em "${texto}"`,
        );
      }
    }
  });
});
