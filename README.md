# Veloria 0.2 — Biblioteca pessoal (Rust + GPUI Community Edition)

Aplicação local para macOS. Interface escura e compacta com grelha responsiva, favoritos, categorias, estados de leitura e pesquisa/filtros.

## Instalação

Requer Rust estável e ferramentas de desenvolvimento Xcode. Na pasta do projeto:

```sh
cargo run
```

> **Nota:** Esta versão preserva o contorno de arranque utilizado após o bloqueio de `MacPlatformState`: a abertura da janela é agendada através de `cx.spawn`. Se já tens uma versão funcional com um arranque diferente, mantém a tua função `main()` e substitui apenas o resto do código.

## Novidades

- **Grelha responsiva:** colunas calculadas pela largura disponível (e ajustadas quando o painel de detalhes é aberto).
- **Títulos contidos:** cartões de largura fixa e textos abreviados para não invadirem as capas vizinhas.
- **Capas locais:** `cover.jpg`, `folder.png`, etc. como antes. Se não existirem, extrai imagens `cover` ou `front` dentro dos EPUB. Nos PDFs, no macOS, tenta gerar uma miniatura com o QuickLook (`qlmanage`).
- **Cache:** em `~/Library/Caches/Veloria/covers` (ou equivalente do sistema). O nome inclui caminho, tamanho e data de modificação do ficheiro, evitando reutilização de capas após alterações.
- **Geração em segundo plano:** só as capas que estão visíveis são colocadas na fila. Para ver as miniaturas acabadas de gerar, clica em **↻ Capas**; os restantes livros são processados à medida que navegas.
- **Filtros e ordenação:** pesquisa; botão **Autor** (nome exato); botão **Formatos** (todos → EPUB → PDF → outros); ordenação por título → autor → recentes → antigos.
- **Organização:** categorias personalizadas, favoritos e estados de leitura continuam guardados no SQLite local.
- **Reindexar:** botão separado para voltar a procurar ficheiros na biblioteca.

## Limitações atuais

- A deteção da capa de EPUB baseia-se em nomes habituais (`cover`, `front`); alguns EPUBs podem indicar a imagem de capa apenas no manifesto OPF e continuar sem capa.
- O QuickLook nem sempre disponibiliza miniaturas para todos os PDFs. Nesses casos aparece a capa tipográfica alternativa.
- A data de ordenação é a **data de modificação do ficheiro** (não a data de publicação ou de criação do livro).
- A leitura/indexação inicial da biblioteca ainda ocorre sincronicamente. A geração de capas é feita numa thread separada.
- A introdução de texto usa diálogos nativos simples; um campo de pesquisa embutido na UI é trabalho futuro.

## Dados e privacidade

A aplicação não envia livros para servidores externos nem altera os documentos. O estado de leitura, favoritos, categorias e caminho da biblioteca são guardados na base de dados local preexistente, preservando os teus dados ao atualizar o código. O diretório `target/` é ignorado pelo Git.

## Compilação

Este pacote foi revisto estaticamente, mas **não foi compilado/testado num Mac neste ambiente**. Corre `cargo check` antes de executar. Se surgir um erro de API GPUI, envia o log completo para corrigirmos com precisão.
