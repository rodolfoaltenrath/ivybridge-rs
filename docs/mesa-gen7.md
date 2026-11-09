# Mesa para Ivy Bridge (HD Graphics 4000, Gen7)

Mapa do código do Mesa 27.0.0-devel (`mesa-main/`) que roda na máquina de
referência. PCI `8086:0162` → `ivb_gt2` (`include/pci_ids/crocus_pci_ids.h`).

## O que é do Mesa e o que não é

| Camada | Onde está | Controla |
|---|---|---|
| OpenGL (crocus) e Vulkan (hasvk) | Mesa, espaço de usuário | Desempenho 3D/compute, compilação de shaders, uso de memória |
| i915 (DRM/KMS) | Kernel Linux | Modos de vídeo, Hz, frequência da GPU (RPS), energia, agendamento |
| Hardware | Silício e placa | Teto de pixel clock no HDMI, banda de memória, clocks máximos |

**A taxa de atualização não passa pelo Mesa.** Para ir além de 60 Hz no Linux,
quem decide é o i915, e o teto de 225 MHz continua valendo
([hardware.md](hardware.md)).

## Componentes usados pela HD 4000

| Componente | Caminho | Linhas C/C++ | Papel |
|---|---|---|---|
| crocus | `src/gallium/drivers/crocus` | ~29,5 mil | Driver OpenGL (Gen4–7.5). Na Gen7 expõe GL/GLSL 4.20 |
| hasvk | `src/intel/vulkan_hasvk` | ~41,4 mil | Driver Vulkan Gen7/8. Avisa: "Ivy Bridge Vulkan support is incomplete" |
| elk | `src/intel/compiler/elk` | ~91,2 mil | Compilador de shaders Gen4–8 (NIR → ISA, vec4 e scalar, SIMD8/16) |
| isl | `src/intel/isl` | ~20,7 mil | Layout de superfícies (tiling, HiZ, MCS) |
| blorp | `src/intel/blorp` | ~13,8 mil | Blits, clears e resolves por shader |
| genxml | `src/intel/genxml/gen70.xml` | (XML) | Definição de comandos e estados da Gen7: a "especificação" lida pelo código |
| dev | `src/intel/dev/intel_device_info.c` | ~6,7 mil | Parâmetros do chip: threads, URB, L3, EUs |

**Fora de escopo** (regra do `mesa-main/AGENTS.md`): `src/intel/compiler/jay`.
O código em `src/intel/compiler/brw` atende Gen9+ e não roda nesta GPU.

## Tetos físicos da máquina

| Recurso | Teto | Observação |
|---|---|---|
| EUs | 16 (Intel), 8 threads cada | Em `intel_device_info_ivb_gt2`: `max_eus_per_subslice = 12`, `max_wm_threads = 172`. Investigar |
| Clock da GPU | 650–1150 MHz | Controlado pelo i915 (RPS). H61 não permite overclock da iGPU |
| FP32 de pico | ~294 GFLOPS | 16 EU × 16 FLOP/clk × 1,15 GHz |
| Memória | DDR3 dual-channel, compartilhada com a CPU | H61 limita a DDR3-1333 (21,3 GB/s). É o gargalo principal de uma iGPU |
| LLC | 8 MB, compartilhado CPU/GPU | `has_llc = true` |

## Onde há espaço para otimizar

1. **Configuração do kernel/sistema** (sem tocar no Mesa): frequência mínima/máxima e boost
   do RPS, governor da CPU, dual-channel e latências da RAM.
2. **Compilador elk**: escolha entre SIMD8 e SIMD16, agendamento de instruções,
   alocação de registradores, spills.
3. **crocus**: custo de CPU por draw call (emissão de estado), partição do cache L3,
   HiZ/fast clear, uploads de buffers.
4. **hasvk**: completar o que falta para Ivy Bridge e reduzir overhead.

Cada mudança só conta se for medida antes e depois, no hardware real, com
correção conferida (piglit/dEQP). Nada pode ser testado no Windows.
