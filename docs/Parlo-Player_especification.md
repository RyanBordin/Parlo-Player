# Especificação de Projeto: Parlo Player — Cliente Desktop Subsonic / OpenSubsonic

> **Objetivo deste Documento (Contexto para IA e Desenvolvimento):**
>
> Este arquivo é a **Fonte Única da Verdade (Single Source of Truth)** para o desenvolvimento do **Parlo Player**, um aplicativo desktop (Windows e Linux) de streaming de música compatível com a API Subsonic / OpenSubsonic. O servidor alvo é o **Navidrome** (e, no futuro, um servidor próprio do autor compatível com a mesma API).
>
> Sempre que este documento for fornecido como contexto, todas as decisões de interface (UI), experiência do usuário (UX), fluxos de navegação, regras de reprodução, integrações de API e tecnologias abaixo devem ser estritamente respeitadas.

**Última atualização:** 18/10/2026

---

## 0. Como usar este documento

* Cada seção é **decidida**, salvo quando marcada como `[Proposta]` ou listada em **Itens em aberto** (Seção 15).
* Decisões não devem ser reabertas sem motivo novo. O histórico está no **Registro de decisões** (Seção 16).
* O que não está neste documento **não deve ser inventado**. Funções fora do escopo da versão atual estão na Seção 14.

---

## 1. Visão Geral e Filosofia de Design (Benchmarking)

O projeto nasce da análise prática de vários players, extraindo os pontos fortes de cada um:

1. **Swing Music (Referência Principal de Estética):**
   * **O que herdamos:** fundo preto puro, interface em preto e branco, cantos arredondados em capas, cards, botões e linhas selecionadas, tipografia limpa com títulos de página grandes em negrito, cor de destaque usada só em detalhes pequenos (ex.: rótulo do tipo do item acima do título). Também herdamos a estrutura da página **Favoritos**, da página de **resultados de busca**, da tela de **Contas** e o encaixe da **capa expandida** na base da barra lateral.
   * **Alcance:** o Swing Music define o visual. A estrutura descrita neste documento (sidebar, Player Bar, Tocando Agora) prevalece.

2. **YouTube Music Desktop App (Referência de Estrutura):**
   * **O que herdamos:** estrutura da barra lateral fixa (navegação no topo + playlists roláveis embaixo), tela expansível "Tocando Agora" (capa grande à esquerda + abas à direita), página de artista com banner largo, lógica de "melhor resultado" na busca, salvar a fila/mix como playlist, botões de controle na miniatura da barra de tarefas do Windows.
   * **O que descartamos:** a barra de reprodução inferior original (capa flutuando de forma desconexa e informações soltas no meio da barra).

3. **Sonixd (Referência de Estrutura Espacial):**
   * **O que herdamos:** cabeçalho das páginas de Álbum/Playlist (capa à esquerda sobre fundo com *blur*, metadados e barra de ações), anatomia da barra de reprodução inferior (capa ancorada à esquerda, transporte e progresso centralizados) e a capa que alterna entre a Player Bar e a barra lateral.
   * **O que descartamos:** avaliação de 5 estrelas, itens de menu desnecessários (Pastas, Gêneros, Rádios) e o estilo datado dos ícones secundários.

4. **Symfonium (Referência de Usabilidade):** app Android favorito do autor. Referência de interface amigável e de **personalização profunda da aparência**.

5. **Feishin e Supersonic (Referências Secundárias):** densidade de tabelas, filtros e recursos de API. Do Feishin herdamos o **indicador da faixa atual** na tracklist (cor de destaque + ícone de play/pause no lugar do número).

---

## 1.1. Identidade Visual

* **Logo:** duas claves de sol em branco, uma invertida em relação à outra, cada uma com um arco em índigo que forma um "P". O par forma "PP" (**P**arlo **P**layer) com simetria de rotação.
* **Versões:** símbolo sozinho (ícone do app, barra lateral) e símbolo + nome "PARLO PLAYER" em serifa (tela de login, Sobre, instalador).
* **Arquivos:** `assets/logo/` com as versões com fundo preto e sem fundo (`-SF`), de símbolo e de símbolo + nome.
* **Paleta base:**

| Uso | Cor | Contraste |
| --- | --- | --- |
| Fundo do app | `#000000` (preto puro) | — |
| Texto principal | `#FFFFFF` | 21:1 sobre o preto |
| **Destaque (fundos):** pílulas, rótulos, botões ativos | `#3E4095` (índigo do logo) | texto branco sobre ele: 8,9:1 |
| **Destaque (texto e ícones)** sobre o preto: faixa atual, rótulos de tipo | `#8183CB` (mesmo matiz, mais claro) | 6,0:1 sobre o preto |

  * O índigo do logo **nunca** é usado como cor de texto sobre o preto (contraste de só 2,4:1).
* **Ícone na bandeja:** versão simplificada, só as claves em branco sobre fundo transparente, para ficar legível em 16 px.
* **Tipografia:** a serifa fica só no logotipo; a interface continua em sans-serif limpa, no estilo Swing Music.
* **Barra lateral:** o símbolo aparece no topo, junto ao nome do app, como no Swing Music.

---

## 2. Princípios do Produto

* **O servidor é a fonte da verdade.** Tudo o que o usuário cria ou marca é salvo no servidor e fica sincronizado com qualquer app Subsonic (Symfonium, Feishin etc.):
  * Favoritos: `star` / `unstar`.
  * Playlists manuais, mixes salvos, recaps e playlists vivas: playlists no servidor (`createPlaylist` / `updatePlaylist`).
  * Fila e posição atual: `savePlayQueue` / `getPlayQueue`.
  * Reproduções: `scrobble` com `submission=true`.
* **Fica só no computador:** downloads, cache de metadados e capas, histórico detalhado de reproduções por data e preferências visuais.
* **Leve e integrado ao sistema:** baixo consumo de memória, controles de mídia do sistema, bandeja, atalhos.

---

## 3. Stack e Arquitetura

| Camada | Tecnologia |
| --- | --- |
| Framework desktop | **Tauri 2** |
| Interface | **Svelte** + TypeScript, HTML/CSS |
| Backend | **Rust** |
| Motor de áudio | **libmpv** (no backend Rust) |
| Controles de mídia do sistema | crate **`souvlaki`** (MPRIS no Linux, SMTC no Windows) |
| Banco local | **SQLite** |
| Senha | Cofre do sistema (Gerenciador de Credenciais no Windows, Secret Service no Linux) |

* O áudio é tocado **pelo backend Rust**, nunca pela tag `<audio>` do webview (evita limitações do WebKitGTK no Linux).
* Botões na miniatura da barra de tarefas do Windows (Thumbnail Toolbar) são implementados no backend Rust via API do Windows.

---

## 4. Arquitetura Global da Janela (App Shell)

A Barra Lateral Esquerda e a Barra de Reprodução Inferior estão **sempre visíveis** em qualquer tela:

```
+-----------------------------------------------------------------------------------+
| [ < ] [ > ]   [ 🔍 Pesquise músicas, álbuns, artistas... ]          [ Perfil/Srv ] | <- Topbar
+------------------+----------------------------------------------------------------+
| [🏠 Início]       |                                                                |
| [❤️ Favoritos]    |                                                                |
| [🎵 Músicas]      |                       VIEWPORT PRINCIPAL                       |
| [💿 Álbuns]       |                                                                |
| [🎤 Artistas]     |                                                                |
| [⬇️ Baixados]     |                                                                |
| [⚙️ Configurações]|                                                                |
| ---------------- |                                                                |
| [ + Nova Playlist]|                                                                |
| ⚡ Recap Out 2026 |                                                                |
| ⚡ Mais escutadas |                                                                |
| 🎵 Playlist User 1|                                                                |
|    (Scroll)      |                                                                |
| [ Capa expandida ]| <- só no modo expandido (Seção 4.3)                            |
+------------------+----------------------------------------------------------------+
| [Capa] Título / Artista      [ |<  ( PLAY )  >| ]     [❤️][+][...]  [🔊][🔁][🔀][▲] | <- Player Bar
|        Álbum             0:25 [======o-----------] 4:00                           |
+-----------------------------------------------------------------------------------+
```

### 4.1. Topbar

* Setas de navegação (voltar/avançar), campo de busca global sempre visível (Seção 6.9) e botão de perfil.
* Atalho **Ctrl+K** foca a busca de qualquer tela.

### 4.2. Barra Lateral Esquerda (Sidebar Persistente)

Dividida em **duas zonas verticais** separadas por uma linha sutil.

#### Zona Superior: Menu de Navegação Principal (Fixo)

Exatamente **7 itens**, nesta ordem:

1. **`Início`** *(Casa)* — Dashboard (Seção 6.1).
2. **`Favoritos`** *(Coração)* — Seção 6.2.
3. **`Músicas`** *(Nota musical)* — Seção 6.3.
4. **`Álbuns`** *(Disco)* — Seção 6.4.
5. **`Artistas`** *(Microfone)* — Seção 6.6.
6. **`Baixados`** *(Seta para baixo)* — Seção 6.8. Sempre disponível, com ou sem conexão.
7. **`Configurações`** *(Engrenagem)* — Seção 6.10.

Sem itens de Pastas, Gêneros ou Rádios.

*Comportamento visual:* o item ativo recebe fundo em pílula arredondada em cinza escuro e ícone/texto em branco brilhante.

#### Zona Inferior: Playlists (Automáticas + Manuais)

* **Cabeçalho:** botão em pílula **`+ Nova playlist`**.
* **Lista rolável** (`overflow-y: auto`) com todas as playlists do usuário: manuais, mixes salvos e playlists automáticas (Seção 11).
* Cada item mostra o título e um subtítulo discreto: `⚡ Playlist automática` para as automáticas; autor/nº de faixas para as demais.
* No modo de capa expandida (Seção 4.3), a lista encolhe para dar espaço à capa.

### 4.3. Barra de Reprodução Inferior (Player Bar Híbrida)

Fixa na base da janela (`height` ~80–90px). Funde a distribuição espacial do Sonixd com os ícones de ação do YouTube Music, no visual do Swing Music.

#### Zona 1: Esquerda — Identificação da Faixa

* **Capa do Álbum** com dois modos, alternados pelo usuário:
  * **Modo compacto (padrão):** miniatura quadrada (~56–64px), cantos arredondados, ancorada na extremidade esquerda. Ao passar o mouse, aparece uma seta **˄**; clicar nela leva ao modo expandido.
  * **Modo expandido:** a capa aparece grande na base da barra lateral, abaixo da lista de playlists, ocupando a largura da sidebar, com cantos arredondados e alinhada às margens (como no Swing Music). Ao passar o mouse, aparece uma seta **˅**; clicar nela devolve a capa à Player Bar.
  * O modo escolhido fica salvo entre sessões.
  * **Nunca** exibir capa flutuante no canto direito.
* **Metadados empilhados** (permanecem na Player Bar nos dois modos):
  1. Título (negrito, cor primária, *text-ellipsis*).
  2. Artista (cinza claro, clicável → página do artista).
  3. Álbum (cinza escuro, clicável → página do álbum).

#### Zona 2: Centro — Transporte e Progresso

* **Linha superior:** Anterior, **Play/Pause** circular em destaque, Próxima.
* **Linha inferior:** tempo decorrido (`tabular-nums`), barra de progresso interativa (clicar e arrastar), tempo total.

#### Zona 3: Direita-Centro — Ações da Música Atual

* **Favoritar** (coração, `star` / `unstar`). Sem 5 estrelas.
* **Adicionar à playlist** (popover rápido).
* **Menu `...`** (Seção 8).

#### Zona 4: Extrema Direita — Áudio, Fila e Expansão

* **Volume:** ícone (clique = mudo) + slider horizontal.
* **Repetir:** Desativado → Repetir fila/álbum → Repetir uma.
* **Aleatório:** liga/desliga o embaralhamento da fila (Seção 5.4).
* **Seta `▲` / `▼`:** abre/recolhe a tela "Tocando Agora" (também ao clicar numa área vazia da barra).

#### Janela estreita

* Largura mínima da janela: **1000px**.
* Em larguras menores, a Player Bar esconde primeiro o slider de volume (fica só o ícone) e depois os botões da Zona 3, que passam para o menu `...`.

### 4.4. Tela "Tocando Agora" (Gaveta Expansível)

Sobe cobrindo a Viewport Principal, mantendo a Player Bar visível.

```
+-----------------------------------------------------------------------------------+
|                                         [ A SEGUIR ]   [ LETRA ]   [ RELACIONADOS ]|
|     +-----------------------------+     ----------------------------------------- |
|     |                             |     Tocando de: Nome do Álbum/Mix   [+ Salvar]|
|     |        CAPA DO ÁLBUM        |     [🔊] 1. Música Atual Tocando        2:45  |
|     |       EM ALTA RESOLUÇÃO     |     [  ] 2. Próxima Música da Fila      2:55  |
|     |         (Quadrada)          |     [  ] 3. Terceira Música             3:36  |
|     +-----------------------------+                                               |
+-----------------------------------------------------------------------------------+
```

* **Coluna esquerda (50–55%):** capa grande, quadrada, centralizada.
* **Coluna direita (45–50%):** 3 abas fixas.
  1. **`A SEGUIR`:**
     * Topo: "Tocando de: [contexto]" (ex.: nome do álbum, "Favoritos", "Busca") e botão **Salvar** (salva a fila como playlist no servidor).
     * Lista rolável da fila. A faixa atual tem ícone indicador. Clicar numa faixa **apenas pula para ela**, sem recriar a fila. Permite reordenar (drag-and-drop) e remover faixas.
  2. **`LETRA`:** letra sincronizada via OpenSubsonic (`getLyricsBySongId`, com destaque do verso atual) e, sem sincronia, letra estática (`getLyrics`).
  3. **`RELACIONADOS`:**
     * **Nível 1 (servidor):** `getSimilarSongs2` e `getTopSongs` (no Navidrome, exigem integração com Last.fm).
     * **Nível 2 (fallback local):** "Mais de [Artista]" (`getArtist` / faixas do mesmo artista) e "Mesmo Gênero ([Gênero])" (`getSongsByGenre` ou `getAlbumList2?type=byGenre`).

---

## 5. Regras de Reprodução e Fila

### 5.1. Clique em uma faixa

* **Clique simples** (não duplo) em qualquer faixa, em qualquer lugar do app (álbum, playlist, Favoritos, Músicas, Baixados, busca, Relacionados), **substitui a fila atual pelo contexto inteiro** onde a faixa está, na ordem exibida (já filtrada e ordenada), e começa a tocar a partir dela.
* Exemplo: ao clicar na 5ª faixa de um álbum, a fila recebe o álbum completo, a reprodução começa na 5ª e o botão **Anterior** volta para a 4ª.
* Não existe opção de tocar uma faixa isolada fora do seu contexto. Para adicionar sem trocar o contexto, usar *Tocar a seguir* / *Adicionar ao fim da fila* (Seção 8).
* **Exceção:** clicar numa faixa dentro da aba `A SEGUIR` apenas pula para ela.

### 5.2. Álbuns, artistas e cards

* **Clique no card** de um álbum (Home, grade de Álbuns, página do artista): abre a página do álbum.
* **Botão Play sobre a capa** (aparece no hover): toca o álbum direto.
* Na página do álbum, a reprodução começa ao clicar numa faixa ou no botão Play do cabeçalho.

### 5.3. Seleção múltipla

* Como o clique simples toca, a seleção usa **Ctrl+clique** (faixas avulsas) e **Shift+clique** (intervalo), sem iniciar reprodução.

### 5.4. Shuffle

* Embaralha a fila a partir da faixa atual e guarda a ordem original, restaurada quando o shuffle é desligado.

### 5.5. Indicador da faixa atual

* Em qualquer tracklist aberta, a faixa tocando aparece na cor de destaque, com ícone de play/pause no lugar do número.

### 5.6. Persistência ao fechar o app

* O app salva: fila completa, contexto de origem, faixa atual, posição exata, shuffle, repeat e volume.
* Ao abrir, restaura tudo exatamente como estava, com a música **pausada**.
* Implementação: `savePlayQueue` / `getPlayQueue` (fila, faixa atual e posição no servidor) + registro local para o que a API não guarda (nome do contexto, shuffle, repeat, volume).

### 5.7. Scrobble

* Envia `scrobble` com `submission=true` quando a faixa passa de **50% ou 4 minutos**, e grava a reprodução no histórico local (Seção 12.4).
* O Navidrome **não** marca reproduções por chamadas a `stream`, só por `scrobble`. Sem isso, "mais escutados", "favoritos esquecidos" e recaps não funcionam.

---

## 6. Especificação de Telas

### 6.1. Início (Dashboard)

* **Filosofia:** para quem não gosta de microgerenciar playlists e prefere ouvir com base no histórico, nos favoritos e na redescoberta da própria biblioteca.
* **6 carrosséis horizontais** (setas `<` `>` no canto superior direito de cada seção), nesta ordem:

| Ordem | Seção | Estilo dos Cards | Fonte de Dados |
| ----- | ----- | ----- | ----- |
| **1** | **Ouvir de novo** | **Cards grandes**, Play centralizado no hover. Últimas músicas e álbuns ouvidos. | Álbuns recentes (`getAlbumList2?type=recent`) + faixas recentes do **histórico local** (a API não tem histórico por faixa). |
| **2** | **Álbuns mais escutados** | Cards padrão (capa, título, artista). | `getAlbumList2?type=frequent`. |
| **3** | **Músicas favoritas** | Cards padrão ou compactos. | `getStarred2` → `song[]`. |
| **4** | **Álbuns favoritos** | Cards padrão. | `getStarred2` → `album[]`. |
| **5** | **Favoritos esquecidos** | Cards padrão. | `getStarred2` ordenado client-side por menor `playCount` e, no desempate, `played` mais antigo (ou ausente). |
| **6** | **Artistas favoritos** | Cards de artista (quadrados). | `getStarred2` → `artist[]`. |

* Seções vazias mostram uma mensagem curta explicando como preenchê-las.

### 6.2. Favoritos

Referência: página Favorites do Swing Music.

* **Cabeçalho:** título "Favoritos" grande e contador (ex.: "3 álbuns • 30 faixas • 1 artista").
* **Seções, nesta ordem:**
  1. **Recentes:** carrossel com os últimos itens favoritados (álbuns, faixas e artistas misturados). Cada card tem um rótulo pequeno colorido acima do título com o tipo (ÁLBUM, FAIXA, ARTISTA).
  2. **Álbuns:** carrossel.
  3. **Músicas:** tabela (coração, capa, título, artista, álbum, duração, `...`) com as mais recentes e link **Ver tudo** para a lista completa com as Ferramentas de lista (Seção 7).
  4. **Artistas:** carrossel.
* Clicar numa faixa toca o contexto "Favoritos" a partir dela.
* Dados: `getStarred2`, ordenados pela data em que foram favoritados.

### 6.3. Músicas

* **Cabeçalho:** título, contador de faixas, botões **Tocar tudo** e **Aleatório**.
* **Barra de ferramentas:** Ferramentas de lista (Seção 7) + seletor de colunas opcionais.
* **Tabela:**
  * Colunas padrão: `#`, capa pequena + título, artista, álbum, duração, coração de favorito.
  * Colunas opcionais: ano, gênero, bitrate, nº de reproduções.
  * Rolagem contínua, sem paginação (lista virtualizada).
  * Ordem padrão: **adicionadas recentemente**.
* Clicar numa faixa toca a lista como está na tela ("Tocando de: Músicas").
* Ordenação, filtros e colunas ficam salvos entre sessões.

### 6.4. Álbuns

* **Cabeçalho:** título, contador e três botões aplicados aos álbuns visíveis (já filtrados e ordenados):
  * **Tocar tudo:** álbuns na ordem da tela, um após o outro.
  * **Álbuns em ordem aleatória:** sorteia a ordem dos álbuns, mantendo a ordem das faixas dentro de cada um.
  * **Mix aleatório:** mistura as faixas dos álbuns visíveis, com no máximo 100 faixas.
* **Barra de ferramentas:** busca rápida, filtros por metadado do álbum (artista, ano, gênero, formato, data de adição, nº de reproduções, favorito, baixado) e seletor "Ordenar por" (nome A–Z, artista, ano, adicionados recentemente, mais tocados, tocados recentemente, aleatório). Padrão: **adicionados recentemente**.
* **Grade:** cards quadrados (capa, título, artista em tom secundário), selos discretos de favorito e baixado, rolagem contínua, colunas ajustadas à largura da janela.
* Clique no card abre o álbum; Play no hover toca; clique direito ou `...` abre o menu de álbum.
* Ordenação e filtros ficam salvos entre sessões.

### 6.5. Página de Álbum e Playlist

* **Cabeçalho (Header com Blur):**
  * **Fundo:** capa ampliada com desfoque pesado (`backdrop-filter: blur`) e gradiente escuro.
  * **Esquerda:** capa nítida (~180–220px), cantos arredondados, sombra leve.
  * **Metadados:** título grande em negrito; linha `ALBUM by [Artista] • [Nº de músicas], [Duração] • [Ano]`; linha secundária com `Added [Data]` e pílulas clicáveis de Artista e Gênero.
* **Barra de ações:** Play (círculo maior), Aleatório, Favoritar, Opções (`...`).
* **Tracklist:** tabela com as Ferramentas de lista (Seção 7) e indicador da faixa atual.
  * Ordem padrão: **álbum** = nº da faixa; **playlist** = ordem salva.
  * Nº da faixa só existe como critério em álbuns.

### 6.6. Artistas

* **Grade de cards quadrados** (foto e nome). Sem foto no servidor, o card usa a capa do álbum mais recente do artista, nunca um ícone genérico.
* **Cabeçalho:** título, contador e **Mix aleatório** (até 100 faixas dos artistas visíveis, salvável como playlist).
* **Barra de ferramentas:** busca rápida, filtros (gênero, favorito, baixado), "Ordenar por" (nome A–Z padrão, mais tocados, adicionados recentemente, nº de álbuns) e filtro de tipo: **artistas de álbum** (padrão) ou **todos os artistas**.

### 6.7. Página do Artista

Referência: página de artista do YouTube Music.

* **Banner largo** com a foto do artista no topo, escurecida embaixo.
* Sobre o banner: nome grande e biografia curta (`getArtistInfo2`) com botão **Mais**.
* **Botões:** Tocar, Aleatório (mix de até 100 faixas), Favoritar, `...`.
* **Seções:**
  1. **Top músicas:** faixas mais tocadas neste servidor (`playCount`); sem reproduções, usa `getTopSongs`.
  2. **Álbuns:** grade, do mais novo para o mais antigo.
  3. **Aparece em:** álbuns de outros artistas em que ele participa.
  4. **Artistas parecidos:** carrossel, quando o servidor fornecer.
* **Tocar** começa pelas Top músicas e segue com o restante das faixas.

### 6.8. Baixados `[v0.2]`

* Mesma estrutura de Favoritos: Recentes (baixados recentemente), Álbuns, Músicas, Artistas.
* Contador com espaço em disco (ex.: "12 álbuns • 148 faixas • 2,3 GB").
* Funciona igual com ou sem conexão.

### 6.9. Busca (Topbar)

Referência: resultados do Swing Music + melhor resultado do YouTube Music.

* **Enquanto digita:** painel abaixo do campo com até 6 resultados misturados (artistas, álbuns, faixas), com rótulo do tipo. Resultados instantâneos do cache local, buscando em todos os metadados. Clicar numa faixa toca; num álbum ou artista, abre a página.
* **Página de resultados (Enter):**
  * Pílulas no topo: **Top**, **Músicas**, **Álbuns**, **Artistas**.
  * Aba **Top:** **Melhor resultado** (card grande com imagem, nome, contagem e rótulo do tipo) à esquerda + **4 primeiras faixas** à direita; abaixo, carrossel de **Artistas** e depois de **Álbuns**.
  * Demais pílulas: só aquele tipo, em lista ou grade completa.
  * Clicar numa faixa toca a lista de músicas encontradas ("Tocando de: Busca").
* Campo vazio e selecionado mostra **buscas recentes**, removíveis.
* Em modo offline, busca só no conteúdo baixado.

### 6.10. Configurações

Página com menu de seções à esquerda. Referência de personalização: Symfonium.

* **Geral**
  * **Aparência:** temas e personalização (cores, logo etc.). `[Detalhes em aberto]`
  * **Perfil:** usuário conectado, nome, senha, servidor (troca de servidor).
  * **Contas:** gestão de usuários no estilo Swing Music (lista com selo ADMIN, "+ Novo usuário", remover). `[Depois]` — ver limitação na Seção 13. No MVP, mostra só o usuário atual.
* **Reprodução e playlists**
  * Tamanho máximo do mix aleatório (padrão: 100).
  * Qualidade de streaming e de download (original ou convertida).
  * Playlists automáticas (Seção 11).
* **Downloads** `[v0.2]`: pasta dos downloads, espaço ocupado, limpar tudo.
* **Biblioteca** `[v0.2]`
  * **Pastas:** origem das músicas (`getMusicFolders`), **rescan** (`startScan`, com opção `fullScan`) e status (`getScanStatus`, inclui data do último scan).
  * **Metadados:** origem dos metadados, no limite do que a API informa.
  * Resumo: nº de faixas, álbuns e artistas.
* **Sobre:** versão, link do GitHub, descrição do projeto, e-mail de contato (opcional).

### 6.11. Primeiro Uso (Login)

* Tela de conexão: URL do servidor, usuário, senha e botão **Testar conexão** (`ping`).
* Um servidor por vez.
* Após o login, tela de progresso da primeira sincronização da biblioteca.

---

## 7. Ferramentas de Lista (Busca, Filtros e Ordenação)

Toda lista de faixas (Músicas, página de Álbum, página de Playlist, "Ver tudo" de Favoritos/Baixados) tem a mesma barra de ferramentas, além da busca global.

* **Busca rápida:** filtra a lista aberta enquanto digita, em **todos os metadados** da faixa.
* **Filtros por metadados** `[avançados na v0.2]`:
  * Montados manualmente: campo + condição + valor (ex.: *Gênero é Rock*, *Ano entre 2000 e 2010*, *Formato é FLAC*, *Bitrate ≥ 320 kbps*, *Favorita*, *Baixada*).
  * Filtros ativos aparecem como pílulas acima da tabela, removíveis com um clique.
  * Campos: título, artista, artista do álbum, álbum, ano, gênero, nº da faixa, nº do disco, duração, formato, bitrate, taxa de amostragem, tamanho, data de adição, nº de reproduções, última reprodução, favorita, baixada, caminho no servidor.
* **Ordenação:** clicar no cabeçalho da coluna ordena; segundo clique inverte.
* **Busca global:** a Topbar também encontra faixas por metadados (compositor, formato etc.).
* **Implementação:** `search3` do Navidrome só faz autocompletar simples em título, artista e álbum. O app mantém os metadados num **cache local SQLite** e faz busca, filtro e ordenação ali.
* O construtor de filtros servirá de base para regras de playlists automáticas criadas pelo usuário (Seção 11.3).

---

## 8. Menus de Opções (`...`)

Cada menu abre pelo botão `...` e pelo **clique direito** no item, com o mesmo conteúdo.

* **Faixa:** Tocar a seguir • Adicionar ao fim da fila • Adicionar à playlist › • Favoritar / Remover dos favoritos • Tocar músicas parecidas (`getSimilarSongs2`) • Ir para o álbum • Ir para o artista • Baixar • Informações do arquivo (formato, bitrate, tamanho, caminho) • *Só em playlist:* Remover desta playlist • *Só em A Seguir:* Remover da fila.
* **Álbum:** Tocar a seguir • Adicionar ao fim da fila • Adicionar à playlist › • Favoritar / Remover • Ir para o artista • Baixar álbum • Informações (ano, gravadora, gênero, formato).
* **Artista:** Tocar a seguir • Adicionar ao fim da fila • Mix aleatório (até 100) • Favoritar / Remover • Baixar todas as faixas • Informações (biografia, nº de álbuns e faixas).
* **Playlist manual:** Tocar a seguir • Adicionar ao fim da fila • Renomear / editar descrição • Tornar pública ou privada • Baixar playlist • Excluir playlist (com confirmação).
* **Playlist automática:** Tocar a seguir • Adicionar ao fim da fila • Salvar como playlist normal (cópia fixa) • Configurar regras (→ Configurações). Não pode ser renomeada nem excluída.

---

## 9. Regra Global: Aleatorizar e Salvar o Mix

* Toda tela que lista música permite aleatorizar: Músicas, Álbuns, Artistas, Favoritos, Baixados e páginas de álbum, artista e playlist.
* Mixes gerados a partir de conjuntos grandes têm **no máximo 100 faixas** (padrão ajustável nas Configurações).
* Qualquer fila gerada pode ser salva como **playlist manual** no servidor pelo botão **Salvar** da aba `A SEGUIR` (`createPlaylist`), com nome sugerido (ex.: "Mix de Álbuns, 18/10/2026") editável. A playlist aparece na barra lateral.

---

## 10. Downloads e Modo Offline `[v0.2]`

* **"Baixar"** guarda uma cópia do arquivo **dentro do app** (pasta de dados do app: `%APPDATA%` no Windows, `~/.local/share` no Linux). O usuário pode encontrar os arquivos, mas não escolhe o destino a cada download.
* Ao tocar uma faixa baixada, o app usa a cópia local.
* **Modo offline automático:** sem conexão com o servidor, o app entra em modo offline, mostra um aviso discreto no topo e exibe **apenas o conteúdo baixado** (nada de itens acinzentados). Ao reconectar, sai do modo offline sozinho.
* A página **Baixados** (Seção 6.8) fica sempre disponível.

---

## 11. Playlists Automáticas `[v0.2]`

Criadas e configuradas em **Configurações → Playlists automáticas**. Aparecem na barra lateral com o subtítulo `⚡ Playlist automática` e são salvas **no servidor** como playlists normais (em outros apps aparecem como playlists comuns).

### 11.1. Recaps (retrospectivas periódicas)

* Uma playlist nova a cada período com as faixas **mais ouvidas** naquele intervalo.
* Períodos: **mensal**, **semestral**, **anual** (o usuário liga os que quiser). Tamanho configurável (20, 50, 100).
* Depois de criado, o recap **não muda** (ex.: "Recap Outubro 2026", "Recap 2026").

### 11.2. Playlists vivas

* Recalculam-se **uma vez por dia** (e ao abrir o app, se a última atualização tiver mais de 24h). Primeiro tipo: **"Mais escutadas"**, com as N faixas mais ouvidas (N configurável).
* **Download automático** opcional por playlist: faixas que entram são baixadas; quando uma faixa sai, o download é apagado, **exceto** se ela estiver em outra playlist baixada ou tiver sido baixada manualmente.

### 11.3. Motor extensível

* Novos tipos devem poder ser adicionados sem reescrever o motor. Cada tipo é uma regra: fonte de dados (histórico, favoritos, metadados), período, ordenação e tamanho.
* Regras criadas pelo usuário reaproveitarão o construtor de filtros (Seção 7).

### 11.4. Implementação e limitação

* A API Subsonic só guarda `playCount` e `played` por faixa, sem histórico por data. O app grava **cada reprodução com data e hora no SQLite local**, desde a v0.1.
* Os recaps só contam reproduções feitas pelo Parlo Player, a partir da instalação, e cada computador calcula com o seu histórico. Solução futura: endpoint de histórico no servidor próprio ou integração com ListenBrainz.

---

## 12. Requisitos Técnicos

### 12.1. Áudio

* **Gapless** e **pré-carregamento** da próxima faixa.
* **ReplayGain** (faixa/álbum), com dados das extensões OpenSubsonic.
* Formatos: FLAC, ALAC, MP3, AAC, Opus, OGG, WAV.
* Qualidade configurável, separada para streaming e download: original ou convertida pelo servidor (`format`, `maxBitRate`).
* `[Depois]` crossfade e equalizador.

### 12.2. Integração com o sistema

* **Linux:** MPRIS (teclas de mídia, widget do desktop, KDE Connect).
* **Windows:** SMTC (painel de mídia, teclas de mídia, tela de bloqueio) e **botões na miniatura da barra de tarefas** (Anterior, Play/Pause, Próxima) `[v0.2]`.
* **Segundo plano:** fechar a janela **não encerra** o app; ele continua tocando, com ícone e controles na bandeja. Na primeira vez, uma notificação avisa o usuário. Para encerrar: "Sair" no menu da bandeja.
* **Atalhos:** Espaço (play/pause), Ctrl+K (busca), Ctrl+→ / Ctrl+← (próxima/anterior), ← / → (±5 s).
* `[Depois]` Discord Rich Presence.

### 12.3. Desempenho (metas)

* Abrir em menos de 2 s com o cache pronto.
* Rolagem fluida com 50 mil faixas (listas virtualizadas).
* Memória bem abaixo de apps Electron como o Feishin.

### 12.4. Cache e dados locais (SQLite)

* Metadados da biblioteca, histórico de reproduções (com data e hora), preferências e estado das playlists automáticas.
* **Um banco por servidor + usuário**, na pasta de dados do app. "Sair" apaga as credenciais, mas **não** o banco, para o histórico de reproduções não se perder.
* **Sincronização em dois níveis:**
  * **Completa:** artistas, álbuns e faixas. Roda no primeiro login, quando a data do último scan do servidor (`getScanStatus`) muda, e no máximo uma vez por dia (para pegar `playCount` alterado por outros apps).
  * **Leve:** favoritos (`getStarred2`) e playlists (`getPlaylists`). Roda a cada abertura do app, porque mudam sem novo scan do servidor (ex.: favoritar no Symfonium).
* Itens removidos do servidor são apagados do cache ao final de uma sincronização completa bem-sucedida.
* Cada registro guarda as colunas usadas pelo app e o **JSON original** da API, para aproveitar campos novos sem ressincronizar.
* Na primeira sincronização, a interface mostra o progresso (Seção 6.11); as seguintes rodam em segundo plano.
* Capas em cache no disco, limite configurável (padrão: 500 MB).

### 12.5. Segurança

* Senha no cofre do sistema, nunca em texto.
* Autenticação Subsonic por token + salt.
* Aceita HTTP na rede local, com aviso discreto quando não for HTTPS.

### 12.6. Erros e estados vazios

* Servidor fora do ar: modo offline.
* Capa que não carrega: placeholder neutro no estilo do app.
* Seções vazias: mensagem curta explicando como preencher.

### 12.7. Idioma e distribuição

* Interface em **português e inglês** desde o início.
* Windows: instalador e versão portátil. Linux: AppImage e Flatpak.
* `[Depois]` atualização automática.

---

## 13. Limitações Conhecidas do Navidrome (API Subsonic)

Verificado na documentação oficial (navidrome.org/docs/developers/subsonic-api):

* **Usuários:** `getUser` / `getUsers` retornam só o usuário autenticado; não há criar/editar/remover usuários. A tela de Contas exige a **API nativa do Navidrome** ou o servidor próprio futuro.
* **Acesso de convidado** e **lista de usuários no login** são recursos de servidor, não de cliente.
* **Busca:** `search2` / `search3` só fazem autocompletar simples (motivo do cache local).
* **Top músicas e similares:** `getTopSongs`, `getSimilarSongs`, `getSimilarSongs2` exigem integração com Last.fm no servidor (motivo do fallback local).
* **`getArtistInfo2`:** exige integrações externas (fotos e biografias podem faltar).
* **Reproduções:** só contam via `scrobble` com `submission=true`.
* **IDs:** sempre strings (nunca converter para inteiro).

---

## 14. Escopo por Versão

### v0.1 (MVP)

* Login, sincronização da biblioteca e cache local.
* App shell completo (sidebar, Topbar com busca, Player Bar com capa compacta e expandida).
* Reprodução com as regras da Seção 5, shuffle, repeat, gapless, retomada e scrobble.
* Histórico local gravado desde o primeiro dia.
* Tocando Agora (A Seguir, Letra, Relacionados).
* Telas: Início, Músicas, Álbuns, Artistas, página do artista, página de álbum/playlist, Favoritos.
* Busca (painel e resultados), busca rápida e ordenação nas listas.
* Menus `...`, favoritar, criar/editar playlists, salvar mix.
* MPRIS/SMTC, bandeja/segundo plano, atalhos.
* Visual Swing Music com um tema escuro fixo; português e inglês.

### v0.2

* Downloads, modo offline e página Baixados.
* Playlists automáticas (recaps e playlists vivas).
* Filtros avançados por metadado.
* Botões na miniatura da barra de tarefas do Windows.
* Configurações de Biblioteca.

### Depois

* Temas e personalização da aparência.
* Contas (gestão de usuários).
* Alternar grade/lista em Álbuns, crossfade, equalizador, Discord Rich Presence, atualização automática.

---

## 15. Itens em Aberto

* [ ] **Aparência e temas:** lista de opções de personalização (cores, logo etc.).
* [ ] **Playlists manuais:** reordenar faixas por drag-and-drop dentro da página da playlist?
* [ ] **Ordem dos carrosséis da Home:** fixa ou configurável pelo usuário?
* [ ] **Novos tipos de playlist automática** além de recaps e "Mais escutadas".

---

## 16. Registro de Decisões

| Data | Decisão |
| --- | --- |
| 17/10/2026 | Clique simples em faixa substitui a fila pelo contexto inteiro; Anterior volta às faixas anteriores; clicar em A Seguir só pula. |
| 17/10/2026 | Fila restaurada ao reabrir, com música pausada (`savePlayQueue`). |
| 17/10/2026 | Servidor alvo: Navidrome (e servidor próprio futuro). |
| 17/10/2026 | Seleção múltipla com Ctrl/Shift + clique. |
| 17/10/2026 | Colunas da tabela de Músicas e comportamento do shuffle. |
| 17/10/2026 | Menus `...` de faixa, álbum, artista e playlists; abrem também pelo clique direito. |
| 17/10/2026 | "Baixar" = cópia offline dentro do app; modo offline automático mostra só o baixado. |
| 17/10/2026 | Sidebar com 7 itens: Baixados entre Artistas e Configurações. |
| 17/10/2026 | Ferramentas de lista (busca em todos os metadados, filtros manuais, ordenação por colunas) em Músicas, Álbum e Playlist. |
| 18/10/2026 | Toda tela permite aleatorizar; mix salvável como playlist; máximo de 100 faixas. |
| 18/10/2026 | Páginas Álbuns, Artistas (cards quadrados), página do artista (estilo YouTube Music). |
| 18/10/2026 | Swing Music como referência principal de estética; YouTube Music fica como referência de estrutura. |
| 18/10/2026 | Capa da faixa atual com modo compacto (Player Bar) e expandido (base da sidebar). |
| 18/10/2026 | Favoritos e Baixados no estilo Swing Music (Recentes, Álbuns, Músicas, Artistas). |
| 18/10/2026 | Busca no estilo Swing Music com melhor resultado. |
| 18/10/2026 | Playlists automáticas: recaps periódicos e playlists vivas diárias, salvas no servidor. |
| 18/10/2026 | O servidor é a fonte da verdade para tudo o que o usuário cria. |
| 18/10/2026 | Estrutura das Configurações (Geral, Reprodução, Downloads, Biblioteca, Sobre). |
| 18/10/2026 | Stack: Tauri 2 + Svelte + Rust + libmpv + souvlaki + SQLite. |
| 18/10/2026 | App roda em segundo plano ao fechar a janela, com aviso ao usuário. |
| 18/10/2026 | Escopo v0.1 / v0.2 / depois. |
| 18/10/2026 | Logo definido (duas claves formando "PP"); índigo do logo vira a cor de destaque. |
| 18/10/2026 | Paleta base: fundo `#000000`, destaque `#3E4095` (fundos) e `#8183CB` (texto/ícones). |
| 18/10/2026 | Sincronização em dois níveis (completa e leve); um banco por servidor + usuário, preservado ao sair. |
