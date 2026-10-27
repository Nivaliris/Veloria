const ui = {
  books: document.querySelector('#books'), empty: document.querySelector('#empty'),
  count: document.querySelector('#totalCount'), status: document.querySelector('#status'),
  folder: document.querySelector('#folderName'), search: document.querySelector('#search'),
  author: document.querySelector('#author'), format: document.querySelector('#format'), sort: document.querySelector('#sort')
};
let books = [];
let folder = null;
let currentView = 'all';
let selectedCategory = '';
let selectedStatus = '';
let visibleLimit = 96;
const categories = JSON.parse(localStorage.getItem('veloria-categories') || '[]');
const bookCategories = JSON.parse(localStorage.getItem('veloria-book-categories') || '{}');
const bookStatuses = JSON.parse(localStorage.getItem('veloria-reading-status') || '{}');
const favoriteIds = new Set(JSON.parse(localStorage.getItem('veloria-favorites') || '[]'));
const openedIds = JSON.parse(localStorage.getItem('veloria-opened') || '[]');

function persist() {
  localStorage.setItem('veloria-favorites', JSON.stringify([...favoriteIds]));
  localStorage.setItem('veloria-opened', JSON.stringify(openedIds));
  localStorage.setItem('veloria-categories', JSON.stringify(categories));
  localStorage.setItem('veloria-book-categories', JSON.stringify(bookCategories));
  localStorage.setItem('veloria-reading-status', JSON.stringify(bookStatuses));
}

const statusLabels = { unread: 'Por ler', reading: 'A ler', finished: 'Concluído' };

function renderCategories() {
  const nav = document.querySelector('#categoryShortcuts');
  const manager = document.querySelector('#categoryManagerList');
  nav.replaceChildren(); manager.replaceChildren();
  for (const category of categories) {
    const count = books.filter(book => (bookCategories[book.id] || []).includes(category)).length;
    const button = document.createElement('button'); button.className = `category-link${selectedCategory === category ? ' chosen' : ''}`;
    const name = document.createElement('span'); name.textContent = `◈  ${category}`;
    const badge = document.createElement('small'); badge.textContent = count;
    button.append(name, badge);
    button.addEventListener('click', () => { selectedCategory = category; setView('category'); });
    nav.append(button);

    const row = document.createElement('div'); row.className = 'category-manager-row';
    const title = document.createElement('span'); title.textContent = `${category} · ${count} livros`;
    const rename = document.createElement('button'); rename.textContent = 'Renomear';
    rename.addEventListener('click', () => {
      const next = window.prompt('Novo nome da categoria:', category)?.trim();
      if (!next || next === category || categories.some(c => c.toLowerCase() === next.toLowerCase())) return;
      categories[categories.indexOf(category)] = next;
      for (const ids of Object.values(bookCategories)) { const at = ids.indexOf(category); if (at !== -1) ids[at] = next; }
      if (selectedCategory === category) selectedCategory = next;
      persist(); renderCategories(); render();
    });
    const remove = document.createElement('button'); remove.textContent = 'Eliminar'; remove.className = 'danger-text';
    remove.addEventListener('click', () => {
      if (!window.confirm(`Eliminar a categoria “${category}”? Os ficheiros dos livros não serão alterados.`)) return;
      categories.splice(categories.indexOf(category), 1);
      for (const ids of Object.values(bookCategories)) { const at = ids.indexOf(category); if (at !== -1) ids.splice(at, 1); }
      if (selectedCategory === category) { selectedCategory = ''; currentView = 'all'; }
      persist(); renderCategories(); render();
    });
    row.append(title, rename, remove); manager.append(row);
  }
}

function renderFilterPanel() {
  const build = (id, items, selected, onSelect) => {
    const host = document.getElementById(id);
    host.replaceChildren();
    for (const [label, count, value] of items) {
      const button = document.createElement('button');
      button.className = `filter-option${selected === value ? ' selected' : ''}`;
      const title = document.createElement('span'); title.textContent = label;
      const amount = document.createElement('small'); amount.textContent = count;
      button.append(title, amount);
      button.addEventListener('click', () => onSelect(selected === value ? '' : value));
      host.append(button);
    }
  };
  const counts = values => [...new Set(values)].map(value => [value, values.filter(item => item === value).length, value]).sort((a,b)=>b[1]-a[1]);
  build('filterAuthors', counts(books.map(book=>book.author)).slice(0, 18), ui.author.value, value => { ui.author.value = value; render(); });
  document.querySelector('#filterAuthorCount').textContent = new Set(books.map(book=>book.author)).size;
  build('filterCategories', categories.map(category => [category, books.filter(book=>(bookCategories[book.id]||[]).includes(category)).length, category]), selectedCategory, value => { selectedCategory=value; setView(value?'category':'all'); });
  build('filterFormats', counts(books.map(book=>book.format)), ui.format.value, value => { ui.format.value=value; render(); });
  build('filterStatuses', Object.entries(statusLabels).map(([value,label])=>[label,books.filter(book=>(bookStatuses[book.id]||'unread')===value).length,value]), selectedStatus, value => { selectedStatus=value; render(); });
}

function checkboxFor(book, category) {
  const label = document.createElement('label'); label.className = 'category-checkbox';
  const checkbox = document.createElement('input'); checkbox.type = 'checkbox'; checkbox.checked = (bookCategories[book.id] || []).includes(category);
  const name = document.createElement('span'); name.textContent = category;
  checkbox.addEventListener('change', () => {
    const assigned = bookCategories[book.id] || (bookCategories[book.id] = []);
    if (checkbox.checked && !assigned.includes(category)) assigned.push(category);
    if (!checkbox.checked) bookCategories[book.id] = assigned.filter(item => item !== category);
    persist(); renderCategories(); render();
  });
  label.append(checkbox, name); return label;
}

async function openBook(book) {
  const opened = await window.library.openBook(book.id);
  if (!opened) { ui.status.textContent = 'Não foi possível abrir este livro com a aplicação predefinida.'; return; }
  const previous = openedIds.indexOf(book.id);
  if (previous !== -1) openedIds.splice(previous, 1);
  openedIds.unshift(book.id);
  openedIds.splice(30);
  persist();
}

function toggleFavorite(book) {
  if (favoriteIds.has(book.id)) favoriteIds.delete(book.id);
  else favoriteIds.add(book.id);
  persist(); render();
}

function detailBook(book) {
  const dialog = document.querySelector('#details');
  const content = document.querySelector('#detailContent');
  content.replaceChildren();
  const layout = document.createElement('div'); layout.className = 'detail-layout';
  const cover = book.cover ? document.createElement('img') : document.createElement('div');
  cover.className = 'detail-cover';
  if (book.cover) { cover.src = book.cover; cover.alt = `Capa de ${book.title}`; }
  else cover.textContent = book.title;
  const info = document.createElement('div'); info.className = 'detail-info';
  const eyebrow = document.createElement('small'); eyebrow.textContent = 'DETALHES DO LIVRO';
  const heading = document.createElement('h2'); heading.textContent = book.title;
  const author = document.createElement('p'); author.textContent = `por ${book.author}`;
  const meta = document.createElement('dl');
  for (const [label, value] of [['Formato', book.format], ['Criado em', new Date(book.created).toLocaleDateString('pt-PT')], ['Ficheiro', book.filename]]) {
    const dt = document.createElement('dt'); dt.textContent = label;
    const dd = document.createElement('dd'); dd.textContent = value;
    meta.append(dt, dd);
  }
  const actions = document.createElement('div'); actions.className = 'detail-actions';
  const statusLabel = document.createElement('label'); statusLabel.className = 'detail-field'; statusLabel.textContent = 'Estado de leitura';
  const statusSelect = document.createElement('select');
  for (const [value, label] of Object.entries(statusLabels)) statusSelect.add(new Option(label, value));
  statusSelect.value = bookStatuses[book.id] || 'unread';
  statusSelect.addEventListener('change', () => { bookStatuses[book.id] = statusSelect.value; persist(); render(); });
  statusLabel.append(statusSelect);
  const categoriesLabel = document.createElement('div'); categoriesLabel.className = 'detail-field'; categoriesLabel.textContent = 'Categorias';
  const checks = document.createElement('div'); checks.className = 'detail-categories';
  if (!categories.length) { const hint = document.createElement('small'); hint.textContent = 'Cria categorias através do botão + na barra lateral.'; checks.append(hint); }
  for (const category of categories) checks.append(checkboxFor(book, category));
  categoriesLabel.append(checks);
  const open = document.createElement('button'); open.className = 'button'; open.textContent = 'Abrir livro ↗'; open.addEventListener('click', () => openBook(book));
  const favorite = document.createElement('button'); favorite.className = 'button secondary'; favorite.textContent = favoriteIds.has(book.id) ? '♥ Remover favorito' : '♡ Adicionar favorito';
  favorite.addEventListener('click', () => { toggleFavorite(book); favorite.textContent = favoriteIds.has(book.id) ? '♥ Remover favorito' : '♡ Adicionar favorito'; });
  actions.append(open, favorite); info.append(eyebrow, heading, author, meta, statusLabel, categoriesLabel, actions); layout.append(cover, info); content.append(layout);
  dialog.showModal();
}

function renderFeatured() {
  const section = document.querySelector('#featured');
  const container = document.querySelector('#featuredCards');
  container.replaceChildren();
  const candidates = [...books].sort((a, b) => b.created - a.created).slice(0, 2);
  section.hidden = currentView !== 'all' || candidates.length === 0;
  for (const book of candidates) {
    const card = document.createElement('button'); card.className = 'featured-card';
    const cover = book.cover ? document.createElement('img') : document.createElement('div'); cover.className = 'featured-cover';
    if (book.cover) { cover.src = book.cover; cover.alt = ''; } else cover.textContent = book.title;
    const copy = document.createElement('div'); copy.className = 'featured-copy';
    const label = document.createElement('small'); label.textContent = '✦ RECENTEMENTE ADICIONADO';
    const title = document.createElement('strong'); title.textContent = book.title;
    const author = document.createElement('span'); author.textContent = book.author;
    copy.append(label, title, author); card.append(cover, copy);
    card.addEventListener('click', () => detailBook(book)); container.append(card);
  }
}

function setView(view) {
  currentView = view;
  visibleLimit = 96;
  if (view !== 'category') selectedCategory = '';
  document.querySelectorAll('[data-view]').forEach(button => button.classList.toggle('active', button.dataset.view === view));
  document.querySelector('#collectionTitle').firstChild.textContent = ({ all: 'Todos os livros ', dashboard: 'Todos os livros ', favorites: 'Os meus favoritos ', recent: 'Adicionados recentemente ', category: `${selectedCategory} ` })[view];
  document.body.classList.toggle('dashboard-view', view === 'dashboard');
  ui.author.value = ''; ui.search.value = ''; render();
}

function optionList(element, values, label) {
  const previous = element.value;
  element.replaceChildren(new Option(label, ''));
  for (const value of values.sort((a, b) => a.localeCompare(b, 'pt'))) element.add(new Option(value, value));
  if (values.includes(previous)) element.value = previous;
}

function render() {
  const query = ui.search.value.trim().toLocaleLowerCase('pt');
  const filtered = books.filter(book =>
    (currentView !== 'favorites' || favoriteIds.has(book.id)) &&
    (currentView !== 'category' || (bookCategories[book.id] || []).includes(selectedCategory)) &&
    (!selectedStatus || (bookStatuses[book.id] || 'unread') === selectedStatus) &&
    (!query || `${book.title} ${book.author}`.toLocaleLowerCase('pt').includes(query)) &&
    (!ui.author.value || book.author === ui.author.value) &&
    (!ui.format.value || book.format === ui.format.value)
  );
  const sort = currentView === 'recent' ? 'created-desc' : ui.sort.value;
  filtered.sort((a, b) => {
    if (sort === 'created-desc') return b.created - a.created;
    if (sort === 'created-asc') return a.created - b.created;
    if (sort === 'modified-desc') return b.modified - a.modified;
    return String(a[sort]).localeCompare(String(b[sort]), 'pt', { sensitivity: 'base' });
  });
  ui.books.replaceChildren();
  const fragment = document.createDocumentFragment();
  for (const book of filtered.slice(0, visibleLimit)) {
    const card = document.createElement('div');
    card.className = 'book';
    card.tabIndex = 0;
    card.setAttribute('role', 'button');
    card.title = `${book.title} — ${book.author}\n${book.filename}`;
    const cover = document.createElement('div');
    cover.className = 'cover';
    if (book.cover) {
      const image = document.createElement('img');
      image.src = book.cover;
      image.alt = `Capa de ${book.title}`;
      image.loading = 'lazy';
      cover.append(image);
    } else {
      const placeholder = document.createElement('div');
      placeholder.className = 'placeholder';
      const mark = document.createElement('div'); mark.className = 'ornament'; mark.textContent = '✦';
      const title = document.createElement('div'); title.className = 'placeholder-title'; title.textContent = book.title;
      placeholder.append(mark, title); cover.append(placeholder);
    }
    const format = document.createElement('span'); format.className = 'format-tag'; format.textContent = book.format;
    cover.append(format);
    const title = document.createElement('div'); title.className = 'book-title'; title.textContent = book.title;
    const author = document.createElement('div'); author.className = 'book-author'; author.textContent = book.author;
    card.append(cover, title, author);
    const favorite = document.createElement('button'); favorite.className = `favorite-toggle${favoriteIds.has(book.id) ? ' is-favorite' : ''}`;
    favorite.title = favoriteIds.has(book.id) ? 'Remover dos favoritos' : 'Adicionar aos favoritos';
    favorite.textContent = favoriteIds.has(book.id) ? '♥' : '♡';
    favorite.addEventListener('click', event => { event.stopPropagation(); toggleFavorite(book); });
    card.append(favorite);
    card.addEventListener('click', () => detailBook(book));
    card.addEventListener('keydown', event => { if (event.target === card && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); detailBook(book); } });
    fragment.append(card);
  }
  ui.books.append(fragment);
  const loadMore = document.querySelector('#loadMore');
  loadMore.hidden = filtered.length <= visibleLimit;
  loadMore.textContent = `Mostrar mais livros (${Math.min(96, filtered.length - visibleLimit)} de ${filtered.length - visibleLimit} restantes)`;
  ui.empty.hidden = filtered.length > 0;
  ui.empty.querySelector('h2').textContent = !folder ? 'A tua biblioteca começa aqui' : books.length ? 'Não encontramos resultados' : 'Ainda não há livros nesta pasta';
  ui.empty.querySelector('p').textContent = !folder ? 'Escolhe a pasta onde guardas os livros. Vamos encontrar ficheiros EPUB, PDF, MOBI, AZW3, CBZ e TXT.' : books.length ? 'Experimenta alterar os filtros ou a pesquisa.' : 'Adiciona livros à pasta escolhida e carrega no botão de atualizar.';
  ui.count.textContent = books.length.toLocaleString('pt-PT');
  document.querySelector('#favoriteCount').textContent = books.filter(book => favoriteIds.has(book.id)).length;
  document.querySelector('#authorCount').textContent = new Set(books.map(book => book.author)).size;
  document.querySelector('#formatCount').textContent = new Set(books.map(book => book.format)).size;
  document.querySelector('#resultsCount').textContent = `${filtered.length} ${filtered.length === 1 ? 'livro' : 'livros'}`;
  renderFeatured();
  renderCategories();
  renderFilterPanel();
  ui.status.textContent = filtered.length === books.length ? '' : `${filtered.length} de ${books.length} livros`;
}

async function reload() {
  ui.status.textContent = 'A analisar os livros…';
  try {
    books = await window.library.scanBooks();
    optionList(ui.author, [...new Set(books.map(b => b.author))], 'Todos os autores');
    optionList(ui.format, [...new Set(books.map(b => b.format))], 'Todos os formatos');
    const shortcuts = document.querySelector('#authorShortcuts'); shortcuts.replaceChildren();
    for (const author of [...new Set(books.map(book => book.author))].sort((a, b) => a.localeCompare(b, 'pt')).slice(0, 8)) {
      const button = document.createElement('button'); button.textContent = `•  ${author}`; button.title = author;
      button.addEventListener('click', () => { setView('all'); ui.author.value = author; render(); });
      shortcuts.append(button);
    }
    render();
  } catch (error) { ui.status.textContent = error.message; }
}

async function chooseFolder() {
  const chosen = await window.library.chooseFolder();
  if (!chosen) return;
  folder = chosen;
  ui.folder.textContent = chosen;
  ui.search.value = '';
  ui.author.value = '';
  ui.format.value = '';
  await reload();
}

for (const id of ['chooseFolder', 'chooseFolderSide', 'chooseFolderEmpty']) document.getElementById(id).addEventListener('click', chooseFolder);
for (const field of [ui.search, ui.author, ui.format, ui.sort]) field.addEventListener(field === ui.search ? 'input' : 'change', render);
document.querySelector('#loadMore').addEventListener('click', () => { visibleLimit += 96; render(); });
document.getElementById('refresh').addEventListener('click', reload);
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => setView(button.dataset.view)));
document.querySelector('#manageCategories').addEventListener('click', () => { renderCategories(); document.querySelector('#categoriesDialog').showModal(); });
document.querySelector('#closeCategories').addEventListener('click', () => document.querySelector('#categoriesDialog').close());
document.querySelector('#categoryForm').addEventListener('submit', event => {
  event.preventDefault();
  const input = document.querySelector('#newCategory');
  const value = input.value.trim();
  if (!value || categories.some(item => item.toLowerCase() === value.toLowerCase())) return;
  categories.push(value); input.value = ''; persist(); render();
});
document.querySelector('#toggleFilters').addEventListener('click', () => document.body.classList.toggle('hide-filters'));
document.querySelector('#clearFilters').addEventListener('click', () => { selectedStatus=''; selectedCategory=''; ui.author.value=''; ui.format.value=''; ui.search.value=''; setView('all'); });
document.querySelector('#closeDetails').addEventListener('click', () => document.querySelector('#details').close());
document.querySelector('#details').addEventListener('click', event => { if (event.target.id === 'details') event.target.close(); });
(async () => { folder = await window.library.getFolder(); ui.folder.textContent = folder || 'Nenhuma pasta selecionada'; await reload(); })();
