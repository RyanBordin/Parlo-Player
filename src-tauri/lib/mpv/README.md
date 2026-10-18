# libmpv (Windows)

Esta pasta recebe a libmpv usada no Windows. Os binários **não** vão para o Git
(veja `src-tauri/.gitignore`): cada desenvolvedor baixa a sua cópia.

Arquivos esperados aqui:

| Arquivo | Para quê |
| --- | --- |
| `libmpv-2.dll` | biblioteca carregada pelo app em execução |
| `mpv.lib` | biblioteca de importação usada pelo linker do MSVC (gerada por `scripts/make-mpv-import-lib.ps1`) |
| `mpv.def` ou `libmpv.dll.a` | vêm no pacote `mpv-dev`; usados para gerar o `mpv.lib` |

Passo a passo completo: [`docs/libmpv-windows.md`](../../../docs/libmpv-windows.md).
