# libmpv no Parlo Player

O Parlo Player toca áudio pela **libmpv**, a biblioteca do player mpv, chamada pelo backend Rust
através da crate [`libmpv2`](https://crates.io/crates/libmpv2). Ela exige **libmpv 2.0 ou mais nova**
(mpv 0.35+).

- **Windows:** a libmpv não vem com o sistema. Cada desenvolvedor baixa uma cópia para
  `src-tauri/lib/mpv/` (seção 1), e o instalador do app leva a DLL junto (seção 3).
- **Linux:** usa a libmpv instalada pelo gerenciador de pacotes (seção 4).

---

## 1. Windows: preparar a libmpv para desenvolvimento

Faça isto uma vez por computador (e de novo quando quiser atualizar a libmpv).

### 1.1. Baixar

1. Abra <https://github.com/shinchiro/mpv-winbuild-cmake/releases>. São os builds de Windows
   automáticos indicados pela comunidade do mpv, publicados quase todo dia.
2. Na release mais recente, abra **Assets** e baixe o arquivo:

   ```
   mpv-dev-x86_64-AAAAMMDD-git-xxxxxxx.7z
   ```

   - **`mpv-dev`**: o pacote para desenvolvedores (DLL + cabeçalhos). Não é o `mpv-x86_64-…`,
     que é o player completo.
   - **`x86_64`** sem `-v3`: a variante `x86_64-v3` exige processadores com AVX2 (de 2013 em
     diante) e não roda em máquinas mais antigas. A `x86_64` roda em qualquer Windows 64 bits.
   - Ignore `i686` (32 bits) e `aarch64` (ARM).

### 1.2. Extrair e copiar

1. Extraia o `.7z`. O Explorador de Arquivos do Windows 11 abre `.7z` direto (botão direito →
   **Extrair tudo**); em versões mais antigas, use o [7-Zip](https://www.7-zip.org/).
2. Copie para **`src-tauri/lib/mpv/`** (a pasta já existe no projeto):
   - `libmpv-2.dll`
   - `mpv.def`, se vier no pacote
   - `libmpv.dll.a`, se vier no pacote

   A pasta `include/` do pacote não é necessária: a crate `libmpv2` já traz os bindings prontos.

### 1.3. Gerar o `mpv.lib`

O compilador do Rust no Windows usa o linker da Microsoft (MSVC), que precisa de um arquivo
`mpv.lib` para saber quais funções a DLL oferece. O pacote não traz esse arquivo, então ele é
gerado a partir do `mpv.def` (ou, se ele não vier, da lista de funções exportadas pela DLL).

Na raiz do repositório, no PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/make-mpv-import-lib.ps1
```

O script encontra sozinho o `lib.exe` e o `dumpbin.exe` das **Visual Studio Build Tools** (as
mesmas que o Rust já usa) e cria `src-tauri/lib/mpv/mpv.lib`. Para fazer à mão, no
"x64 Native Tools Command Prompt for VS 2022":

```bat
cd src-tauri\lib\mpv
lib /def:mpv.def /name:libmpv-2.dll /out:mpv.lib /machine:x64
```

### 1.4. Conferir

A pasta deve ficar assim (os arquivos `.exp` e `.def` gerados podem ficar):

```
src-tauri/lib/mpv/
├── README.md        (vai para o Git)
├── libmpv-2.dll     (ignorado pelo Git)
├── mpv.def          (ignorado pelo Git)
└── mpv.lib          (ignorado pelo Git)
```

Depois disso, `cargo test` e `npm run tauri dev` funcionam normalmente.

> **Por que não vai para o Git?** A DLL tem dezenas de megabytes e é um binário de terceiros.
> O `src-tauri/.gitignore` ignora tudo em `lib/mpv/` exceto o `README.md`.

---

## 2. Como o build encontra a libmpv

Tudo fica em `src-tauri/build.rs` e só vale quando o alvo é Windows:

1. Informa ao linker a pasta `src-tauri/lib/mpv/` (`cargo:rustc-link-search`), onde está o
   `mpv.lib`.
2. Copia a `libmpv-2.dll` para `src-tauri/target/<perfil>/` e `target/<perfil>/deps/`. O Windows
   procura DLLs na pasta do executável, então é assim que o `npm run tauri dev` (executável em
   `target/debug/`) e o `cargo test` (executáveis em `target/debug/deps/`) a encontram.
3. Se `mpv.lib` ou `libmpv-2.dll` estiver faltando, o build para com uma mensagem apontando para
   este documento.

---

## 3. App final (instalador do Windows)

O arquivo `src-tauri/tauri.windows.conf.json` (que o Tauri só aplica no Windows) declara a DLL em
`bundle.resources`. No Windows, o Tauri instala os recursos **na pasta do executável**, que é onde
o Windows procura as DLLs ao abrir o app.

> A verificação do instalador será feita na etapa de distribuição: o `npm run tauri build`
> baixa as ferramentas do instalador (NSIS/WiX) na primeira vez.

---

## 4. Linux

No Linux, o app usa a libmpv do sistema; nada vai em `src-tauri/lib/mpv/`.

| Distribuição | Para compilar | Para rodar |
| --- | --- | --- |
| Debian / Ubuntu | `sudo apt install libmpv-dev` | `libmpv2` |
| Fedora | `sudo dnf install mpv-devel` (repositório RPM Fusion) | `mpv-libs` |
| Arch | `sudo pacman -S mpv` | `mpv` |

- **Pacote `.deb`:** `src-tauri/tauri.linux.conf.json` declara `libmpv2` como dependência, então
  o gerenciador de pacotes instala a libmpv junto com o app.
- **AppImage e Flatpak** (etapa de distribuição): o AppImage precisa levar a libmpv dentro dele;
  o Flatpak precisa de um módulo do mpv no manifesto.

---

## 5. Licença (decidir antes de publicar o instalador)

- A crate `libmpv2` é **LGPL-2.1**, e o app a usa por link dinâmico.
- Os builds do shinchiro são compilados com componentes **GPL**. Distribuir essa DLL dentro do
  instalador de um app MIT pode trazer obrigações da GPL para a distribuição.
- Para desenvolver, nada muda. Antes de publicar um instalador, decidir entre: aceitar as
  condições da GPL na distribuição, ou usar um build da libmpv compilado em modo LGPL
  (`-Dgpl=false`).

Isto é um lembrete técnico, não uma orientação jurídica.

---

## 6. Problemas comuns

| Sintoma | Causa provável |
| --- | --- |
| Build para com "libmpv not found … docs/libmpv-windows.md" | `mpv.lib` ou `libmpv-2.dll` faltando em `src-tauri/lib/mpv/` (seções 1.2 e 1.3) |
| `LNK1181: cannot open input file 'mpv.lib'` | o `mpv.lib` não foi gerado (seção 1.3) |
| Ao abrir, o Windows avisa que `libmpv-2.dll` não foi encontrada | a DLL não está ao lado do `.exe`; rode o build de novo para o `build.rs` copiar |
| `STATUS_DLL_NOT_FOUND` ao rodar `cargo test` | idem, em `target/debug/deps/` |
| O app abre, mas não toca nada e não mostra erro | variante errada: use `x86_64`, não `x86_64-v3`, em CPUs sem AVX2 |
