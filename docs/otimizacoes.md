# Backlog de otimizações

Candidatos a otimização, cada um com hipótese, forma de medir e onde mexer.
**Nenhum está comprovado:** são pontos de partida para o ciclo da
[metodologia](metodologia.md). A ordem é a sugerida para atacar: primeiro o que
é barato e mede o terreno.

Estados: ⬜ não iniciado · 🔬 em investigação · ✅ aceito · ❌ descartado

## Nível 0: sistema (sem mudar código)

### O1. Memória em dual-channel e na velocidade máxima ⬜
- **Hipótese:** uma iGPU é limitada pela banda de memória; single-channel
  corta essa banda pela metade.
- **Medir:** `dmidecode -t memory`; glmark2 antes/depois de reposicionar os pentes.
- **Onde:** hardware e BIOS.

### O2. Frequência da GPU e política de boost ⬜
- **Hipótese:** o RPS do i915 pode demorar a subir o clock em cargas curtas.
- **Medir:** `intel_gpu_top` durante os benchmarks; comparar frequência livre
  com `gt_min_freq_mhz` fixado em RP0.
- **Onde:** `/sys/class/drm/card*/gt_*_freq_mhz` (kernel i915).

### O3. Compilação do Mesa para esta CPU ⬜
- **Hipótese:** `-march=ivybridge` (AVX) e LTO reduzem o custo de CPU do driver,
  que pesa em jogos com muitas draw calls.
- **Medir:** glmark2 e um teste limitado por CPU (muitas draw calls); `perf stat`.
- **Onde:** `meson setup -Dc_args=-march=ivybridge -Dcpp_args=-march=ivybridge -Db_lto=true`.
  Depois, avaliar PGO (`-Db_pgo=generate`/`use`).

## Nível 1: parâmetros do driver

### O4. Contagem de EUs em `intel_device_info_ivb_gt2` ⬜
- **Fato:** o Mesa declara `max_eus_per_subslice = 12`; a Intel especifica
  16 EUs na HD 4000.
- **Hipótese:** se algum cálculo de despacho usa esse número, a GPU pode estar
  subutilizada. Ou pode ser só convenção de contagem, sem efeito.
- **Medir:** primeiro, achar onde o valor é usado (`grep max_eus_per_subslice`);
  só testar mudança se ele influenciar despacho de threads.
- **Onde:** `mesa-main/src/intel/dev/intel_device_info.c`.

### O5. Partição do cache L3 no crocus ⬜
- **Hipótese:** a Gen7 permite dividir o L3 entre URB, dados e SLM; uma divisão
  escolhida por carga (como o iris faz na Gen8+) pode render mais que uma fixa.
- **Medir:** `INTEL_DEBUG=l3` para ver a configuração; benchmarks de compute e de
  texturas pesadas.
- **Onde:** `crocus_state.c`, `crocus_screen.c`, `src/intel/common/intel_l3_config.c`.

### O6. HiZ e fast clear ⬜
- **Hipótese:** já ajudam, mas podem estar sendo desativados em casos comuns.
- **Medir:** comparar com `INTEL_DEBUG=nohiz` / `nofc` para saber quanto valem;
  `INTEL_DEBUG=perf` para achar resolves desnecessários.
- **Onde:** `crocus_resolve.c`, `crocus_clear.c`, `src/intel/blorp`.

## Nível 2: compilador de shaders (elk)

### O7. Escolha de SIMD8 / SIMD16 ⬜
- **Hipótese:** a heurística que escolhe a largura do fragment shader pode errar
  nesta GPU (SIMD16 com spills fica mais lento que SIMD8).
- **Medir:** `INTEL_DEBUG=fs` para estatísticas; comparar `fs8` e `fs16` por cena.
- **Onde:** `src/intel/compiler/elk/elk_fs.cpp`.

### O8. Spills e pressão de registradores ⬜
- **Hipótese:** shaders com spills acessam a memória, que é o recurso mais escasso.
- **Medir:** `INTEL_DEBUG=spill_fs,spill_vec4,reg-pressure`; contar spills nos jogos-alvo.
- **Onde:** alocador de registradores e agendador do elk.

### O9. Passagens NIR específicas para Gen7 (Rust) ⬜
- **Hipótese:** padrões que sobram depois das otimizações genéricas podem ser
  reescritos de forma mais barata para a ISA da Gen7.
- **Medir:** estatísticas de instruções por shader + benchmarks.
- **Onde:** nova passagem em Rust sobre `mesa-main/src/compiler/rust` (bindings
  de NIR), chamada pelo pipeline do elk.

## Nível 3: custo de CPU do driver

### O10. Emissão de estado no crocus ⬜
- **Hipótese:** reemissão de estado redundante a cada draw call custa CPU.
- **Medir:** `perf record -g` num teste com muitas draw calls; contar comandos
  com `INTEL_DEBUG=bat-stats`.
- **Onde:** `crocus_state.c`, `crocus_draw.c`, `crocus_batch.c`.

## Nível 4: Vulkan (hasvk)

### O11. Completar o suporte a Ivy Bridge ⬜
- **Fato:** o hasvk avisa que o suporte a Ivy Bridge está incompleto.
- **Medir:** lista de falhas do dEQP-VK no M1; cada correção é um item.
- **Onde:** `mesa-main/src/intel/vulkan_hasvk`.

## Limpeza de build

### O12. Tirar `brw` e `jay` (Gen9+) do build ⬜
- **Ganho:** tempo de compilação, não desempenho.
- **Onde:** `src/intel/compiler/meson.build`, `src/intel/blorp/meson.build`,
  `src/intel/tools/meson.build`. O diretório `jay` não pode ser alterado
  (`mesa-main/AGENTS.md`); só a forma como o build o inclui.
