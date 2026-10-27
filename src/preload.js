const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('library', {
  getFolder: () => ipcRenderer.invoke('get-folder'),
  chooseFolder: () => ipcRenderer.invoke('choose-folder'),
  scanBooks: () => ipcRenderer.invoke('scan-books'),
  openBook: id => ipcRenderer.invoke('open-book', id)
});
