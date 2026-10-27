# Veloria Library

Biblioteca pessoal offline para macOS e Windows, feita em Electron. Escolhe uma pasta e a aplicação identifica os livros e apresenta-os numa interface de catálogo escura com detalhes em roxo, grelha compacta de capas, filtros laterais, categorias personalizadas, favoritos, estados de leitura e um dashboard opcional. Abre os ficheiros na aplicação predefinida.

## Executar

Instala o [Node.js](https://nodejs.org/) (versão LTS). Na pasta do projeto:

```bash
npm install
npm start
```

Para gerar uma aplicação instalável, no respetivo sistema operativo:

```bash
npm run build:mac
# ou, no Windows
npm run build:win
```

O instalador é criado em `dist/`. Uma aplicação macOS sem assinatura digital poderá exigir autorização manual nas definições de segurança.

## Organização recomendada

```text
Biblioteca/
  Ursula K. Le Guin/
    A Wizard of Earthsea/
      A Wizard of Earthsea.epub
      cover.jpg
  Jane Austen/
    Pride and Prejudice/
      livro.pdf
      capa.png
```

Também podes guardar `Biblioteca/Autor/livro.epub` ou livros diretamente na pasta principal. A aplicação lê EPUB, PDF, MOBI, AZW3, CBZ e TXT, incluindo subpastas até oito níveis. A capa é procurada na pasta do livro (`cover.jpg`, `capa.png`, `folder.jpg` ou com o nome do ficheiro); para EPUB tenta também encontrar uma imagem de capa dentro do ficheiro. Para PDFs sem imagem separada, aparece uma capa estilizada com o título.

**Datas:** "Mais recentes" ordena pela data de criação do ficheiro no sistema operativo, não pela data de publicação do livro. Os livros não são copiados nem alterados; a pasta selecionada fica guardada nas preferências locais.

**Favoritos:** São guardados localmente na aplicação. Ao clicar numa capa abres os detalhes do livro; usa o botão «Abrir livro» para o abrir no leitor instalado. A área «Em destaque» mostra os ficheiros mais recentemente criados na coleção.

**Categorias:** O botão `+` ao lado de «Categorias» permite criar, renomear ou eliminar categorias. Para atribuir uma ou mais categorias, abre os detalhes de um livro e seleciona-as. As categorias e os estados de leitura são guardados localmente e não alteram as tuas pastas. A barra lateral e o painel direito permitem filtrar por categorias, autores, formatos e estados.

**Coleções grandes:** A grelha mostra os primeiros 96 resultados e permite carregar progressivamente mais livros. Para os EPUB, tenta extrair também o título e o autor dos metadados internos. A leitura inicial de milhares de ficheiros pode demorar um pouco, dependendo do disco.

**Limitação atual:** PDFs sem `cover.jpg`/`capa.png` ou imagem com o nome do ficheiro continuam a usar capa estilizada. A extração automática da primeira página PDF ainda não está implementada.

## Próximas melhorias possíveis

- Leitura de metadados internos (autor, título, ano, série e ISBN) de EPUB/PDF.
- Extração da primeira página PDF como capa e leitura de `metadata.opf`.
- Favoritos, progresso de leitura, etiquetas e várias bibliotecas.
