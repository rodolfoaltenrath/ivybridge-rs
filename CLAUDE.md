# Instruções para agentes (Claude Code e similares)

## Contexto

Projeto para levar a Intel HD Graphics 4000 (i7-3770, Ivy Bridge/Gen7, placa
H61) ao limite físico no Linux, otimizando o driver do Mesa ponto a ponto. Leia
o [README](README.md) e a [metodologia](docs/metodologia.md) antes de propor
mudanças.

## Regras

- **Idioma:** português do Brasil nas respostas, documentos, mensagens de
  commit e textos voltados ao usuário. Identificadores de código em inglês.
- **Medir antes de afirmar.** Não declarar ganho de desempenho sem resultado em
  `bench/results/`. Hipóteses são chamadas de hipóteses.
- **Uma mudança por experimento**, em branch `opt/<nome>`, seguindo
  [docs/metodologia.md](docs/metodologia.md).
- **`mesa-main/` segue o `mesa-main/AGENTS.md`:**
  - não alterar `src/intel/compiler/jay`;
  - commits com código escrito por IA levam o trailer `Generated-by: LLM`;
  - nunca abrir merge requests nem comentar no GitLab do Mesa.
- **Escopo do Mesa:** só crocus, hasvk, elk e as dependências deles. Não
  reintroduzir drivers removidos (ver `mesa-main/IVYBRIDGE.md`).
- **Segurança:** nada de aplicar modos de vídeo, frequências ou parâmetros de
  energia sem forma de reverter; restaurar valores originais ao fim de
  experimentos.

## Comandos

```sh
# Mesa (Linux)
cd mesa-main
meson setup build --prefix="$HOME/.local/mesa-ivb" --libdir=lib -Dbuildtype=release
ninja -C build install
source ../tools/linux/mesa-env.sh    # usa o Mesa compilado só no terminal atual
glxinfo -B; vulkaninfo --summary

# ivyprobe (Rust, qualquer plataforma)
cargo test
cargo clippy --all-targets
cargo fmt
```

## Onde está cada coisa

| Caminho | Conteúdo |
|---|---|
| `mesa-main/src/gallium/drivers/crocus` | driver OpenGL |
| `mesa-main/src/intel/vulkan_hasvk` | driver Vulkan |
| `mesa-main/src/intel/compiler/elk` | compilador de shaders Gen4–8 |
| `mesa-main/src/intel/dev/intel_device_info.c` | parâmetros da GPU (`ivb_gt2`) |
| `mesa-main/src/compiler/rust` | bindings Rust de NIR, para passagens novas |
| `docs/otimizacoes.md` | backlog: atualizar o estado de cada item |
| `bench/results/` | um arquivo por experimento |
| `src/`, `tests/` | ivyprobe (diagnóstico de EDID/modos) |
