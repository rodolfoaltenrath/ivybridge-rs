# Resultados de medição

Um arquivo por experimento em `bench/results/`, com o nome
`AAAA-MM-DD-<id>.md`, onde `<id>` é o item do
[backlog](../docs/otimizacoes.md) ou `baseline` (ex.: `2026-11-16-baseline.md`,
`2026-11-18-O5-l3.md`).

Resultados negativos e inconclusivos também são registrados.

## Modelo

Copie para `bench/results/` e preencha:

````markdown
# <id>: <título>

- **Data:** AAAA-MM-DD
- **Branch / commit:** opt/<nome> @ <hash>
- **Hipótese:** <o que deveria acontecer e por quê>
- **Resultado:** ✅ aceito | ❌ descartado | ⚠️ inconclusivo

## Ambiente

| Item | Valor |
|---|---|
| Kernel | `uname -r` |
| Distribuição | |
| Mesa | commit do projeto + flags do `meson setup` |
| GPU | frequência fixa em ___ MHz / livre |
| CPU governor | performance |
| RAM | dual-channel, ___ MT/s |
| Resolução | |

## Medições (5 execuções, mediana)

| Benchmark | Referência | Mudança | Diferença | Variação (mín–máx) |
|---|---|---|---|---|
| glmark2 (score) | | | | |
| vkmark (score) | | | | |

## Correção

| Suíte | Referência | Mudança |
|---|---|---|
| piglit (rápido) | X falhas | Y falhas |
| dEQP-VK (rápido) | X falhas | Y falhas |

## Notas

<o que foi observado, gargalos, próximos passos>
````
