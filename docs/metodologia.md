# Metodologia

Uma otimização só entra no projeto se for **medida**, se for **maior que o
ruído** e se **não quebrar nada**. Sem número, não tem otimização.

## O ciclo

```
hipótese → medição de referência → mudança → medição → testes de correção → decisão
```

1. **Hipótese.** Escrita antes de mexer no código: o que muda, por que deveria
   ficar mais rápido e em qual benchmark isso deve aparecer. Ex.: "o crocus usa
   uma partição fixa do L3; uma partição com mais espaço para dados deve
   acelerar shaders de compute em X%".
2. **Referência.** Medir o estado atual, no mesmo dia e nas mesmas condições
   (ver Protocolo).
3. **Mudança.** Uma por vez, num branch próprio. Duas mudanças juntas
   impedem saber qual delas teve efeito.
4. **Medição.** Mesmos benchmarks, mesmo protocolo.
5. **Correção.** Rodar os testes de regressão (ver Testes de correção).
6. **Decisão.** Aceitar, descartar ou investigar mais, e registrar o resultado
   em `bench/results/`, **inclusive quando der errado**: uma hipótese
   descartada também é conhecimento.

## Protocolo de medição

Benchmark de GPU varia com temperatura, frequência e o que mais está rodando.
Para comparar, as condições precisam ser iguais:

| Condição | Como garantir |
|---|---|
| Frequência da GPU fixa | `gt_min_freq_mhz = gt_max_freq_mhz` (ex.: 1150) durante a medição; restaurar depois |
| CPU sem economia de energia | governor `performance` (`sudo cpupower frequency-set -g performance`) |
| Sem sincronização vertical | GL: `vblank_mode=0`; vkmark: `--present-mode immediate` |
| Sem compositor interferindo | sessão X sem compositor, ou benchmark em tela cheia |
| Mesma resolução | anotar sempre |
| Máquina ociosa | fechar navegador e demais programas |
| Repetições | **5 execuções**, usar a **mediana**; anotar mínimo e máximo |

Ruído: se a diferença entre referência e mudança for menor que a variação entre
execuções do mesmo build, o resultado é **inconclusivo**, não ganho.

Também vale medir com a frequência livre (comportamento real do dia a dia), mas
como medição separada e identificada.

### Benchmarks

| Ferramenta | Mede | Comando-base |
|---|---|---|
| glmark2 | OpenGL, cenas variadas | `vblank_mode=0 glmark2 --fullscreen` |
| vkmark | Vulkan, cenas variadas | `vkmark --present-mode immediate` |
| jogo ou benchmark real | o que importa no uso | definido no M1 e mantido fixo |
| `intel_gpu_top` | ocupação e frequência da GPU durante o teste | em paralelo, para entender gargalos |
| `perf` | custo de CPU do driver | `perf record -g -- <benchmark>` |

### Ferramentas de diagnóstico do próprio Mesa

Variável `INTEL_DEBUG` (lista completa em `mesa-main/src/intel/dev/intel_debug.c`):

| Valor | Para quê |
|---|---|
| `perf` | avisos de caminhos lentos no driver |
| `fs`, `vs`, `gs`, `cs` | imprime o código gerado e as estatísticas de cada shader |
| `spill_fs`, `spill_vec4`, `reg-pressure` | investigar falta de registradores |
| `fs8`, `fs16` | forçar a largura SIMD do fragment shader, para comparar |
| `nohiz`, `nofc` | desligar HiZ / fast clear, para medir quanto eles ajudam |
| `bat`, `bat-stats` | decodificar os batches de comandos enviados à GPU |
| `l3` | mostrar a configuração de L3 escolhida |

## Testes de correção

Uma otimização que deixa algo errado na tela não é otimização.

| Driver | Suíte | Quando |
|---|---|---|
| crocus (GL) | piglit (via deqp-runner), subconjunto rápido | a cada mudança |
| hasvk (Vulkan) | dEQP-VK (via deqp-runner), subconjunto rápido | a cada mudança |
| ambos | suítes completas | antes de juntar ao `main` mudanças no compilador ou em estado da GPU |

No M1 é gravada a **lista de referência** de falhas existentes. Daí em diante,
qualquer teste que passava e passa a falhar bloqueia a mudança.

## Rust ou C

| Situação | Linguagem |
|---|---|
| Ajuste dentro de código existente do Mesa (estado, flags, heurísticas) | C, a mudança mínima |
| Componente novo com fronteira clara: passagem de otimização NIR, peça do compilador elk | Rust, usando `mesa-main/src/compiler/rust` (bindings de NIR) |
| Ferramentas de medição, análise de resultados, scripts complexos | Rust, fora do `mesa-main` |

A troca de linguagem, por si só, não é otimização: Rust entra onde o
componente é novo ou reescrito por outro motivo medido.

## Git

- Branch por experimento: `opt/<nome-curto>` (ex.: `opt/l3-config`).
- Um commit por mudança lógica, com mensagem em português explicando o porquê.
- Commits que alteram `mesa-main/` com código escrito por IA levam o trailer
  `Generated-by: LLM`; com código do usuário assistido por IA,
  `Assisted-by: LLM` (regra do `mesa-main/AGENTS.md`).
- O resultado vai em `bench/results/` no mesmo branch, para ficar junto do código
  que o gerou.
- Nada é enviado ao repositório upstream do Mesa.

## Segurança

- Mudanças em modos de vídeo só com reversão automática por tempo.
- Experimentos de frequência e energia: restaurar os valores originais ao fim.
- Travamentos da GPU (`GPU HANG` no `dmesg`) são resultado a registrar, não
  algo a ignorar: anexar o `dmesg` e o arquivo de erro em `/sys/class/drm/card*/error`.
