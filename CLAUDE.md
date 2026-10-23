# Parlo Player

Cliente desktop (Windows e Linux) para servidores Subsonic / OpenSubsonic, com foco no **Navidrome**.

## Fonte da verdade

- A especificação completa está em `docs/Parlo-Player_especification.md`. **Leia antes de qualquer tarefa de interface, reprodução ou API.**
- Não invente funções que não estão na especificação. Se algo não estiver definido, pergunte.
- Respeite os marcadores de versão: só implemente o que é **v0.1 (MVP)**, a menos que eu peça outra coisa.

## Como trabalhar comigo

- Estou aprendendo com este projeto. Trabalhe em **passos pequenos** e explique o que vai fazer e por quê antes de fazer.
- Antes de mudanças grandes (nova dependência, nova pasta, mudança de arquitetura), apresente um plano e espere minha aprovação.
- Converse comigo em **português**. Código, nomes de variáveis, commits e comentários em **inglês**.
- Ao terminar uma etapa, diga como eu testo o resultado.

## Stack

- **Tauri 2** (app desktop)
- **Svelte + TypeScript** na interface (`src/`)
- **Rust** no backend (`src-tauri/`)
- **libmpv** para áudio, sempre no backend Rust (nunca usar a tag `<audio>` do webview)
- **souvlaki** para controles de mídia do sistema (MPRIS no Linux, SMTC no Windows)
- **SQLite** para cache de metadados, histórico de reproduções e preferências
- Senha no cofre do sistema (crate `keyring`), nunca em texto

## Arquitetura (backend Rust)

`src-tauri/` é um workspace Cargo. O que não depende de Tauri nem de libmpv fica no crate `crates/parlo-core/`, que será compartilhado com o app Android:

- `parlo-core/src/subsonic/` — cliente da API Subsonic (autenticação token + salt, endpoints, tipos)
- `parlo-core/src/player/` — fila, shuffle, repeat, retomada, tempo ouvido, ReplayGain
- `parlo-core/src/db/` — SQLite (cache da biblioteca, histórico, estado)
- `parlo-core/src/settings.rs` — preferências do app

O app desktop fica em `src-tauri/src/`:

- `player/` — reprodução com libmpv, gapless
- `media_controls/` — integração com MPRIS/SMTC e bandeja
- `commands.rs` — comandos Tauri expostos para a interface

A interface só conversa com o backend por **comandos e eventos Tauri**.

## Regras importantes da API (Navidrome)

- IDs são sempre **strings**. Nunca converter para número.
- Reproduções só contam com `scrobble` + `submission=true` (enviar após 50% da faixa ou 4 minutos).
- `search3` é limitado: busca, filtros e ordenação são feitos no cache local SQLite.
- `getTopSongs` e `getSimilarSongs2` dependem de Last.fm no servidor: sempre ter fallback local.
- Tudo o que o usuário cria (favoritos, playlists, fila) é salvo **no servidor**.

## Comandos

- Rodar em modo desenvolvimento: `npm run tauri dev`
- Testes do backend: `cargo test --workspace` (dentro de `src-tauri/`)
- Checagem da interface: `npm run check`

## Ordem de construção (v0.1)

1. Esqueleto do projeto rodando (janela vazia)
2. Cliente Subsonic em Rust com testes (`ping`, autenticação)
3. Tela de login e senha no cofre do sistema
4. Sincronização da biblioteca para o SQLite
5. Reprodução com libmpv (tocar, pausar, fila)
6. App shell (sidebar, Topbar, Player Bar) com o visual Swing Music
7. Telas, na ordem: Álbuns, página de álbum, Músicas, Artistas, Favoritos, Início, Busca
8. Tocando Agora, menus `...`, MPRIS/SMTC, bandeja
