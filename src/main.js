const { app, BrowserWindow, dialog, ipcMain, shell } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const AdmZip = require('adm-zip');

const EXTENSIONS = new Set(['.epub', '.pdf', '.mobi', '.azw3', '.cbz', '.txt']);
const IMAGES = ['cover.jpg', 'cover.jpeg', 'cover.png', 'folder.jpg', 'folder.png', 'capa.jpg', 'capa.png'];
let selectedFolder = null;
let knownBooks = new Set();

function settingsFile() { return path.join(app.getPath('userData'), 'settings.json'); }

async function restoreFolder() {
  try {
    const data = JSON.parse(await fs.readFile(settingsFile(), 'utf8'));
    if (typeof data.folder === 'string') selectedFolder = data.folder;
  } catch { /* First launch. */ }
}

async function saveFolder(folder) {
  await fs.mkdir(app.getPath('userData'), { recursive: true });
  await fs.writeFile(settingsFile(), JSON.stringify({ folder }), 'utf8');
}

function embeddedEpubCover(filename) {
  try {
    const zip = new AdmZip(filename);
    const entries = zip.getEntries();
    const images = entries.filter(e => !e.isDirectory && /\.(png|jpe?g|webp)$/i.test(e.entryName));
    const likely = images.find(e => /(^|\/)(cover|capa)[^/]*\.(png|jpe?g|webp)$/i.test(e.entryName));
    const candidate = likely || (images.length === 1 ? images[0] : null);
    if (!candidate || candidate.header.size > 5 * 1024 * 1024) return null;
    const ext = path.extname(candidate.entryName).toLowerCase();
    const mime = ext === '.png' ? 'image/png' : ext === '.webp' ? 'image/webp' : 'image/jpeg';
    return `data:${mime};base64,${candidate.getData().toString('base64')}`;
  } catch { return null; }
}

function epubMetadata(filename) {
  try {
    const zip = new AdmZip(filename);
    const opf = zip.getEntries().find(entry => !entry.isDirectory && /\.opf$/i.test(entry.entryName));
    if (!opf) return {};
    const xml = opf.getData().toString('utf8');
    const decode = value => value.replace(/<[^>]*>/g, '').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").trim();
    const match = tag => xml.match(new RegExp(`<dc:${tag}\\b[^>]*>([\\s\\S]*?)<\\/dc:${tag}>`, 'i'));
    return { title: match('title') ? decode(match('title')[1]) : undefined, author: match('creator') ? decode(match('creator')[1]) : undefined };
  } catch { return {}; }
}

async function coverFor(bookPath) {
  const dir = path.dirname(bookPath);
  const basename = path.basename(bookPath, path.extname(bookPath));
  for (const file of [`${basename}.jpg`, `${basename}.png`, ...IMAGES]) {
    try {
      const data = await fs.readFile(path.join(dir, file));
      if (data.byteLength > 5 * 1024 * 1024) continue;
      const mime = path.extname(file).toLowerCase() === '.png' ? 'image/png' : 'image/jpeg';
      return `data:${mime};base64,${data.toString('base64')}`;
    } catch { /* Try next cover. */ }
  }
  return path.extname(bookPath).toLowerCase() === '.epub' ? embeddedEpubCover(bookPath) : null;
}

async function scan(folder) {
  const books = [];
  const root = await fs.realpath(folder);
  async function walk(dir, depth) {
    if (depth > 8 || books.length >= 5000) return;
    let entries;
    try { entries = await fs.readdir(dir, { withFileTypes: true }); }
    catch { return; }
    for (const entry of entries) {
      if (entry.name.startsWith('.')) continue;
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(file, depth + 1);
      else if (entry.isFile() && EXTENSIONS.has(path.extname(entry.name).toLowerCase())) {
        try {
          const stat = await fs.stat(file);
          const parts = path.relative(root, file).split(path.sep);
          const metadata = path.extname(entry.name).toLowerCase() === '.epub' ? epubMetadata(file) : {};
          const firstFolder = parts[0];
          const author = metadata.author || (parts.length > 1 && !/^[_\.]/.test(firstFolder) && !/^(livros?|books?|ebooks?|biblioteca|ficção|romance|fantasia|desenvolvimento)/i.test(firstFolder) ? firstFolder : 'Autor desconhecido');
          const filenameTitle = path.parse(entry.name).name.replace(/[_]+/g, ' ').trim();
          const parent = parts.length >= 3 ? parts[parts.length - 2] : '';
          const title = metadata.title || (/^(livro|book|ebook|ficheiro|documento)(\s*\d+)?$/i.test(filenameTitle) && parent ? parent : filenameTitle);
          books.push({ id: file, title, author, filename: entry.name, format: path.extname(entry.name).slice(1).toUpperCase(), created: stat.birthtimeMs, modified: stat.mtimeMs, cover: await coverFor(file) });
        } catch { /* A file may disappear during scan. */ }
      }
    }
  }
  await walk(root, 0);
  knownBooks = new Set(books.map(book => book.id));
  return books;
}

function createWindow() {
  const win = new BrowserWindow({
    width: 1320, height: 850, minWidth: 880, minHeight: 600,
    backgroundColor: '#0d0b14', title: 'Veloria Library',
    webPreferences: { preload: path.join(__dirname, 'preload.js'), contextIsolation: true, nodeIntegration: false, sandbox: true }
  });
  win.loadFile(path.join(__dirname, 'index.html'));
}

app.whenReady().then(async () => {
  await restoreFolder();
  ipcMain.handle('get-folder', () => selectedFolder);
  ipcMain.handle('choose-folder', async () => {
    const choice = await dialog.showOpenDialog({ properties: ['openDirectory'], defaultPath: selectedFolder || app.getPath('documents') });
    if (choice.canceled || !choice.filePaths[0]) return selectedFolder;
    selectedFolder = choice.filePaths[0];
    await saveFolder(selectedFolder);
    return selectedFolder;
  });
  ipcMain.handle('scan-books', async () => {
    if (!selectedFolder) return [];
    try { return await scan(selectedFolder); }
    catch { throw new Error('Não foi possível aceder à pasta selecionada. Escolhe outra pasta.'); }
  });
  ipcMain.handle('open-book', async (_event, filename) => {
    if (typeof filename !== 'string' || !knownBooks.has(filename)) return false;
    return (await shell.openPath(filename)) === '';
  });
  createWindow();
  app.on('activate', () => { if (BrowserWindow.getAllWindows().length === 0) createWindow(); });
});
app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
