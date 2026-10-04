import type { ModuleDestination } from "./modules.ts";

/**
 * O que um destino mostra enquanto a sua `SPEC-D` não existe.
 *
 * Esta tela é uma **decisão de honestidade**, não um placeholder. Um menu com nove
 * entradas que abre tela vazia ensina que o produto está pronto e não está; e a
 * diferença entre "ainda não foi construído" e "não existe" é informação que a
 * usuária precisa para não procurar o que não está lá.
 *
 * O texto que aparece aqui é a mesma informação do `roadmap.md`: o nome do
 * destino, a `SPEC-D` que falta escrever e a fase que o entrega.
 */
export function DestinoNaoImplementado({
  modulo,
  destino,
}: {
  modulo: string;
  destino: ModuleDestination;
}) {
  const bloqueios = destino.bloqueadoPor;

  return (
    <section className="mx-auto flex h-full max-w-2xl flex-col justify-center gap-4 px-8 py-16">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{modulo}</p>
      <h1 className="text-2xl font-semibold">{destino.rotulo}</h1>

      <p className="text-sm text-muted-foreground">
        Este destino existe no registro de módulos, mas ainda não foi implementado. Ele é trabalho
        declarado, não uma tela que quebrou.
      </p>

      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
        <dt className="text-muted-foreground">Rota</dt>
        <dd className="font-mono text-xs">{destino.rota}</dd>

        <dt className="text-muted-foreground">Especificação</dt>
        <dd className="font-mono text-xs">{destino.spec ?? `SPEC-D a definir para ${modulo}`}</dd>

        <dt className="text-muted-foreground">Decisão de produto</dt>
        <dd className="font-mono text-xs">{destino.modoTelaCheia ? "modo de tela cheia" : "—"}</dd>
      </dl>

      {bloqueios && bloqueios.length > 0 ? (
        <div className="rounded-[8px] border border-border bg-card/60 px-4 py-3">
          <p className="text-sm font-medium">Bloqueado por decisão em aberto</p>
          <ul className="mt-1 list-disc pl-5 text-sm text-muted-foreground">
            {bloqueios.map((motivo) => (
              <li key={motivo}>{motivo}</li>
            ))}
          </ul>
          <p className="mt-2 text-xs text-muted-foreground">
            Item `[A]` da spec referencial. Agente não resolve decisão em aberto.
          </p>
        </div>
      ) : null}
    </section>
  );
}
