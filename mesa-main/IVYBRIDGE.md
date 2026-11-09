# Mesa reduzido para Ivy Bridge (HD Graphics 4000)

Fork local do Mesa 27.0.0-devel com apenas o necessário para a Intel Gen7:

- **crocus**: OpenGL (`src/gallium/drivers/crocus`)
- **hasvk**: Vulkan (`src/intel/vulkan_hasvk`)
- **elk**: compilador de shaders Gen4–8 (`src/intel/compiler/elk`)
- isl, blorp, genxml, dev, common, perf, decoder (`src/intel/`)
- infraestrutura compartilhada: NIR/GLSL/SPIR-V (`src/compiler`), estado GL
  (`src/mesa`), gallium auxiliary, DRI, EGL, GLX, GBM, runtime Vulkan e WSI

Base: Mesa `main` em 27.0.0-devel (novembro de 2026). Para recuperar algo
removido, use o upstream: https://gitlab.freedesktop.org/mesa/mesa

## O que foi removido

- Drivers de outras GPUs: AMD, NVIDIA (nouveau/NVK), Qualcomm, ARM (Panfrost/Lima),
  Apple, Broadcom, Vivante, Imagination, Microsoft, gfxstream, KosmicKrisp
- Gallium: todos os drivers exceto crocus (inclusive iris, que é Gen8+, i915,
  que é Gen3, llvmpipe/softpipe e zink); winsys, frontends e targets que não
  sejam crocus, sw e dri
- Intel Gen9+: `src/intel/vulkan` (anv), `shaders`, `executor`, `nullhw-layer`
- CI (inclusive `bin/ci`), documentação, Android, `src/tool/gamma`, `src/virtio`
  exceto `virtio-gpu` e `vdrm`, e os `.gitignore` do upstream (as regras úteis
  foram para o `.gitignore` da raiz do projeto)

## O que continua no build por exigência dele

- `src/intel/compiler/brw` (Gen9+): o meson o compila sempre, e o blorp
  depende dele. Tirá-lo do build é o próximo passo de limpeza.
- `src/intel/compiler/jay`: idem. O `AGENTS.md` do Mesa proíbe alterar essa
  pasta, então ela fica intacta.

## Mudanças nos arquivos de build

- `meson.options`: `gallium-drivers` só aceita `crocus`, `vulkan-drivers` só
  aceita `intel_hasvk` (e esses são os padrões); `llvm` desativado por padrão
- `src/meson.build`: removidos `include_directories` de AMD/llvmpipe/RADV
- `src/gallium/auxiliary/meson.build`: removido include de `drivers/zink`
- `src/intel/meson.build`, `src/tool/meson.build`: removidos subdirs apagados

As referências restantes a pastas removidas estão todas em blocos condicionais
que ficam falsos com essas opções.

## Compilar (Linux)

```sh
meson setup build -Dbuildtype=release -Dplatforms=x11,wayland
ninja -C build
# usar sem instalar:
meson devenv -C build glxinfo -B
```

**Ainda não compilado:** a limpeza foi verificada por análise estática no
Windows. O primeiro `meson setup` no Linux é o teste de verdade.
