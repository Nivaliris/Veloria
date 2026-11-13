mod catalog;
mod storage;
mod covers;

use std::{path::PathBuf, collections::{HashMap, HashSet}, sync::mpsc::{self, Sender}};
use gpui::{div, img, prelude::*, px, rgb, App, Context, IntoElement, QuitMode, Render, Window, WindowOptions};
use catalog::Book;
use storage::Storage;

const BG: u32 = 0x101014;
const PANEL: u32 = 0x16161C;
const SURFACE: u32 = 0x1C1B23;
const BORDER: u32 = 0x2B2934;
const TEXT: u32 = 0xEEEAF3;
const MUTED: u32 = 0x888393;
const ACCENT: u32 = 0xA78BFA;

#[derive(Debug, Clone, PartialEq)]
enum View { All, Favorites, Reading, Finished, Category(String) }
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sort { Title, Author, Recent, Oldest }
#[derive(Debug, Clone, Copy, PartialEq)]
enum FormatFilter { All, Epub, Pdf, Other }

struct Veloria {
    store: Storage,
    root: Option<PathBuf>,
    books: Vec<Book>,
    categories: Vec<String>,
    view: View,
    search: String,
    page: usize,
    selected: Option<PathBuf>,
    sort: Sort,
    format_filter: FormatFilter,
    author_filter: Option<String>,
    cover_cache: HashMap<PathBuf, Option<PathBuf>>,
    queued_covers: HashSet<PathBuf>,
    cover_tx: Sender<PathBuf>,
    message: String,
}
impl Veloria {
    fn new() -> Self {
        let store = Storage::open().expect("Could not open Veloria database");
        let root = store.root();
        let categories = store.categories().unwrap_or_default();
        let (cover_tx, cover_rx) = mpsc::channel::<PathBuf>();
        std::thread::spawn(move || {
            while let Ok(path)=cover_rx.recv() {
                let _ = covers::resolve(&path);
            }
        });
        let mut this = Self { store, root, books: Vec::new(), categories, view:View::All,
            search: String::new(), page: 0, selected: None, sort:Sort::Title, format_filter:FormatFilter::All, author_filter:None, cover_cache:HashMap::new(), queued_covers:HashSet::new(), cover_tx, message:String::new() };
        this.reload(); this
    }
    fn reload(&mut self) {
        self.books = self.root.as_ref().map(|r| catalog::scan(r)).unwrap_or_default();
        self.cover_cache.clear();
        self.queued_covers.clear();
        for b in &mut self.books {
            let (fav, status, categories) = self.store.book_data(&b.path);
            b.favorite = fav; b.status = status; b.categories = categories;
        }
        self.categories = self.store.categories().unwrap_or_default();
        self.page = 0;
        self.message = format!("{} livros encontrados", self.books.len());
    }
    fn filtered(&self) -> Vec<usize> {
        let q = self.search.to_lowercase();
        let mut indexes:Vec<_> = self.books.iter().enumerate().filter(|(_,b)| {
            let matches_search = q.is_empty() || b.title.to_lowercase().contains(&q) || b.author.to_lowercase().contains(&q);
            let matches_view = match &self.view {
                View::All => true, View::Favorites => b.favorite,
                View::Reading => b.status == "A ler", View::Finished => b.status == "Concluído",
                View::Category(c) => b.categories.contains(c),
            };
            matches_search && matches_view
        }).map(|(i,_)| i).collect();
        indexes.retain(|i| {
            let book = &self.books[*i];
            let format_ok = match self.format_filter { FormatFilter::All=>true, FormatFilter::Epub=>book.format=="EPUB", FormatFilter::Pdf=>book.format=="PDF", FormatFilter::Other=>book.format!="EPUB" && book.format!="PDF" };
            format_ok && self.author_filter.as_ref().is_none_or(|a| &book.author == a)
        });
        match self.sort {
            Sort::Title => indexes.sort_by(|a,b| self.books[*a].title.to_lowercase().cmp(&self.books[*b].title.to_lowercase())),
            Sort::Author => indexes.sort_by(|a,b| self.books[*a].author.to_lowercase().cmp(&self.books[*b].author.to_lowercase()).then(self.books[*a].title.cmp(&self.books[*b].title))),
            Sort::Recent => indexes.sort_by(|a,b| self.books[*b].modified.cmp(&self.books[*a].modified)),
            Sort::Oldest => indexes.sort_by(|a,b| self.books[*a].modified.cmp(&self.books[*b].modified)),
        }
        indexes
    }
    fn button(&self, id: &str, label: impl Into<String>, active: bool) -> gpui::Stateful<gpui::Div> {
        div().id(id.to_string()).px_3().py_2().rounded_md()
            .bg(rgb(if active { 0x302542 } else { SURFACE }))
            .text_color(rgb(if active { 0xD9C5FF } else { 0xBBB4C5 }))
            .text_sm().cursor_pointer().hover(|s| s.bg(rgb(0x342B42)))
            .child(label.into())
    }
    fn nav(&self,id:&str,label:impl Into<String>,active:bool) -> gpui::Stateful<gpui::Div> {
        div().id(id.to_string()).w_full().px_3().py_2().rounded_md()
            .bg(rgb(if active { 0x2B223C } else { PANEL }))
            .text_color(rgb(if active { 0xDAC7FF } else { 0xA49CAD }))
            .text_sm().cursor_pointer().hover(|s| s.bg(rgb(0x282332)))
            .child(label.into())
    }
    fn cycle_sort(&mut self) {
        self.sort = match self.sort { Sort::Title=>Sort::Author, Sort::Author=>Sort::Recent, Sort::Recent=>Sort::Oldest, Sort::Oldest=>Sort::Title };
        self.page=0;
    }
    fn cycle_format(&mut self) {
        self.format_filter=match self.format_filter { FormatFilter::All=>FormatFilter::Epub, FormatFilter::Epub=>FormatFilter::Pdf, FormatFilter::Pdf=>FormatFilter::Other, FormatFilter::Other=>FormatFilter::All };
        self.page=0;
    }
    fn choose_author(&mut self) {
        let choices: Vec<String> = {
            let mut a:Vec<String>=self.books.iter().map(|b|b.author.clone()).collect();
            a.sort_by_key(|s|s.to_lowercase());a.dedup();a
        };
        let initial = self.author_filter.as_deref().unwrap_or("");
        if let Some(text)=tinyfiledialogs::input_box("Filtrar por autor", "Nome exato, ou vazio para limpar:",initial) {
            let t=text.trim();
            self.author_filter=if t.is_empty() {None} else {choices.iter().find(|a| a.eq_ignore_ascii_case(t)).cloned()};
            if !t.is_empty() && self.author_filter.is_none(){self.message="Autor não encontrado. Filtro limpo.".into();}
            self.page=0;
        }
    }
    fn cover_for(&mut self, book: &Book) -> Option<PathBuf> {
        if let Some(p)=&book.cover {return Some(p.clone());}
        // A cheap cache lookup happens on each draw, so newly generated covers
        // appear on the next user action / refresh without rescanning books.
        if let Some(p)=covers::cached(&book.path) {
            self.cover_cache.insert(book.path.clone(), Some(p.clone()));
            return Some(p);
        }
        if self.queued_covers.insert(book.path.clone()) {
            let _=self.cover_tx.send(book.path.clone());
        }
        None
    }
    fn refresh_covers(&mut self) {
        self.cover_cache.clear();
        self.message="Capas atualizadas. Miniaturas em processamento podem surgir ao atualizar novamente.".into();
    }
    fn choose_folder(&mut self) {
        if let Some(root)=rfd::FileDialog::new().set_title("Escolhe a pasta da biblioteca").pick_folder() {
            match self.store.set_root(&root) {
                Ok(()) => { self.root=Some(root); self.selected=None; self.reload(); },
                Err(e) => self.message=format!("Erro ao guardar pasta: {e}")
            }
        }
    }
    fn search_dialog(&mut self) {
        if let Some(s)=tinyfiledialogs::input_box("Pesquisar livros", "Título ou autor (vazio para limpar):", &self.search) {
            self.search=s.trim().to_string(); self.page=0;
        }
    }
    fn new_category(&mut self) {
        if let Some(s) = tinyfiledialogs::input_box("Nova categoria", "Nome da coleção:", "") {
            let name=s.trim();
            if !name.is_empty() { if let Err(e)=self.store.add_category(name) { self.message=e.to_string(); } self.categories=self.store.categories().unwrap_or_default(); }
        }
    }
    fn rename_category(&mut self, old: &str) {
        if let Some(s)=tinyfiledialogs::input_box("Renomear categoria", "Novo nome:", old) {
            let name=s.trim();
            if !name.is_empty() && name != old {
                if let Err(e)=self.store.rename_category(old,name) { self.message=e.to_string(); }
                self.categories=self.store.categories().unwrap_or_default(); self.reload();
                self.view=View::Category(name.into());
            }
        }
    }
    fn remove_category(&mut self, old: &str) {
        if tinyfiledialogs::message_box_yes_no("Eliminar categoria", &format!("Eliminar '{old}'? Os ficheiros não serão apagados."), tinyfiledialogs::MessageBoxIcon::Question, tinyfiledialogs::YesNo::No) == tinyfiledialogs::YesNo::Yes {
            if let Err(e)=self.store.remove_category(old) { self.message=e.to_string(); }
            self.view=View::All; self.reload();
        }
    }
    fn toggle_favorite(&mut self,path:&PathBuf) {
        if let Some(book)=self.books.iter_mut().find(|b| &b.path==path) {
            let next=!book.favorite;
            if self.store.set_favorite(path,next).is_ok() { book.favorite=next; }
        }
    }
    fn change_status(&mut self,path:&PathBuf) {
        if let Some(book)=self.books.iter_mut().find(|b| &b.path==path) {
            let next=match book.status.as_str() { "Por ler"=>"A ler", "A ler"=>"Concluído", _=>"Por ler" };
            if self.store.set_status(path,next).is_ok() { book.status=next.into(); }
        }
    }
    fn toggle_assignment(&mut self,path:&PathBuf, category:&str) {
        if let Some(book)=self.books.iter_mut().find(|b| &b.path==path) {
            let has=book.categories.iter().any(|s|s==category);
            if self.store.toggle_category(path,category,has).is_ok() {
                if has { book.categories.retain(|s|s!=category); }
                else { book.categories.push(category.into()); }
            }
        }
    }
}

impl Render for Veloria {
    fn render(&mut self, window:&mut Window, cx:&mut Context<Self>) -> impl IntoElement {
        let filtered=self.filtered();
        let total=filtered.len();
        let visible:Vec<usize>=filtered.into_iter().take((self.page+1)*112).collect();
        let viewport = window.viewport_size();
        let panel_width = if self.selected.is_some() {266.0} else {0.0};
        let available = (viewport.width.as_f32() - 220.0 - panel_width - 52.0).max(170.0);
        let columns = ((available + 12.0) / 151.0).floor().max(1.0) as usize;
        // Resolve covers only for the currently displayed page. Re-render uses the in-memory cache.
        let mut covers_by_path=HashMap::new();
        for &idx in &visible {
            let book=self.books[idx].clone();
            covers_by_path.insert(book.path.clone(), self.cover_for(&book));
        }
        let title=match &self.view { View::All=>"Todos os livros".to_string(),View::Favorites=>"Favoritos".into(),View::Reading=>"A ler".into(),View::Finished=>"Concluídos".into(),View::Category(c)=>c.clone() };
        let selected_book=self.selected.as_ref().and_then(|p| self.books.iter().find(|b|&b.path==p)).cloned();

        // Left navigation: simple and restrained, like a native desktop tool.
        let mut side=div().w(px(220.)).h_full().flex().flex_col().gap_2().p_4().bg(rgb(PANEL))
            .border_r_1().border_color(rgb(BORDER))
            .child(div().pb_5().pt_2().text_xl().font_weight(gpui::FontWeight::BOLD).text_color(rgb(TEXT)).child("✦  veloria"))
            .child(div().text_xs().text_color(rgb(MUTED)).pb_2().child("BIBLIOTECA"))
            .child(self.nav("all","▦  Todos os livros",self.view==View::All).on_click(cx.listener(|s,_,_,cx|{s.view=View::All;s.page=0;cx.notify();})))
            .child(self.nav("favorites","♡  Favoritos",self.view==View::Favorites).on_click(cx.listener(|s,_,_,cx|{s.view=View::Favorites;s.page=0;cx.notify();})))
            .child(self.nav("reading","◷  A ler",self.view==View::Reading).on_click(cx.listener(|s,_,_,cx|{s.view=View::Reading;s.page=0;cx.notify();})))
            .child(self.nav("finished","✓  Concluídos",self.view==View::Finished).on_click(cx.listener(|s,_,_,cx|{s.view=View::Finished;s.page=0;cx.notify();})))
            .child(div().mt_6().mb_2().flex().justify_between().text_xs().text_color(rgb(MUTED))
                .child("COLEÇÕES")
                .child(div().id("new-category").cursor_pointer().text_color(rgb(ACCENT)).child("+ Nova")
                    .on_click(cx.listener(|s,_,_,cx|{s.new_category();cx.notify();}))));
        for (n,cat) in self.categories.iter().enumerate() {
            let cat_value=cat.clone();
            let active=self.view==View::Category(cat_value.clone());
            side=side.child(self.nav(&format!("cat-{n}"),format!("▫  {cat}"),active)
                .on_click(cx.listener(move |s,_,_,cx|{s.view=View::Category(cat_value.clone());s.page=0;cx.notify();})));
        }
        side=side.child(div().flex_1())
            .child(div().text_xs().text_color(rgb(MUTED)).child(self.root.as_ref().map(|p|p.display().to_string()).unwrap_or_else(||"Sem pasta selecionada".into())))
            .child(self.button("folder", "Escolher pasta",false).on_click(cx.listener(|s,_,_,cx|{s.choose_folder();cx.notify();})));

        let header=div().w_full().flex().items_center().justify_between().pb_5()
            .child(div().flex().flex_col().gap_1()
                .child(div().text_2xl().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(title))
                .child(div().text_sm().text_color(rgb(MUTED)).child(format!("{total} livros · {}", self.message))))
            .child(div().flex().gap_2()
                .child(self.button("search", if self.search.is_empty() { "⌕  Pesquisar".to_string() } else { format!("⌕  {}  ×",self.search) },false)
                    .on_click(cx.listener(|s,_,_,cx|{s.search_dialog();cx.notify();})))
                .child(self.button("sort",match self.sort {Sort::Title=>"Título A–Z",Sort::Author=>"Autor A–Z",Sort::Recent=>"Recentes",Sort::Oldest=>"Antigos"},false)
                    .on_click(cx.listener(|s,_,_,cx|{s.cycle_sort();cx.notify();})))
                .child(self.button("format",match self.format_filter {FormatFilter::All=>"Formatos",FormatFilter::Epub=>"EPUB",FormatFilter::Pdf=>"PDF",FormatFilter::Other=>"Outros"},false)
                    .on_click(cx.listener(|s,_,_,cx|{s.cycle_format();cx.notify();})))
                .child(self.button("author",if self.author_filter.is_some() {"Autor ✓"} else {"Autores"},false)
                    .on_click(cx.listener(|s,_,_,cx|{s.choose_author();cx.notify();})))
                .child(self.button("refresh","↻ Capas",false).on_click(cx.listener(|s,_,_,cx|{s.refresh_covers();cx.notify();})))
                .child(self.button("rescan","Reindexar",false).on_click(cx.listener(|s,_,_,cx|{s.reload();cx.notify();}))));

        let mut grid=div().w_full().flex().flex_col().gap_4();
        if self.root.is_none() {
            grid=grid.child(div().p_8().rounded_lg().bg(rgb(SURFACE)).text_color(rgb(TEXT))
                .child("Escolhe uma pasta para começares a tua biblioteca.\nEx.: Biblioteca / Autor / Livro / ficheiro.epub"));
        } else if visible.is_empty() {
            grid=grid.child(div().p_8().text_color(rgb(MUTED)).child("Nenhum livro nesta vista."));
        }
        // Fixed batches prevent 3,000+ elements from being rendered at once.
        for (row_index,chunk) in visible.chunks(columns).enumerate() {
            let mut row=div().flex().gap_3();
            for (col,book_index) in chunk.iter().enumerate() {
                let book=&self.books[*book_index];
                let key=format!("book-{row_index}-{col}");
                let path=book.path.clone();
                let mut cover=div().w(px(139.)).h(px(194.)).rounded_md().bg(rgb(0x282135))
                    .overflow_hidden().flex().items_center().justify_center();
                if let Some(file)=covers_by_path.get(&book.path).and_then(|p|p.as_ref()) {
                    cover=cover.child(img(file.clone()).w_full().h_full());
                } else {
                    cover=cover.child(div().p_3().flex().flex_col().gap_3().items_center()
                        .child(div().text_2xl().text_color(rgb(ACCENT)).child("✦"))
                        .child(div().w(px(115.)).overflow_hidden().text_center().text_xs().text_color(rgb(0xD5C9E5)).child(shorten(&book.title, 38))));
                }
                row=row.child(div().id(key).w(px(139.)).flex().flex_col().gap_1().cursor_pointer()
                    .child(cover)
                    .child(div().w(px(139.)).overflow_hidden().text_sm().text_color(rgb(TEXT)).child(shorten(&book.title, 23)))
                    .child(div().w(px(139.)).overflow_hidden().text_xs().text_color(rgb(MUTED)).child(shorten(&book.author, 24)))
                    .on_click(cx.listener(move |s,_,_,cx|{s.selected=Some(path.clone());cx.notify();})));
            }
            grid=grid.child(row);
        }
        if visible.len()<total {
            grid=grid.child(self.button("more",format!("Mostrar mais · {} de {total}",visible.len()),false)
                .on_click(cx.listener(|s,_,_,cx|{s.page+=1;cx.notify();})));
        }
        let main_panel=div().flex_1().h_full().p_6().flex().flex_col().bg(rgb(BG))
            .child(header)
            .child(div().id("catalog-scroll").flex_1().overflow_y_scroll().child(grid));

        let right=if let Some(book)=selected_book {
            let path=book.path.clone();
            let path2=path.clone();
            let path3=path.clone();
            let detail_image=covers_by_path.get(&book.path).and_then(|p|p.clone()).or_else(||covers::cached(&book.path)).or(book.cover.clone());
            let preview = if let Some(file)=detail_image {
                div().w(px(150.)).h(px(210.)).rounded_md().overflow_hidden().bg(rgb(SURFACE)).child(img(file).w_full().h_full())
            } else {
                div().w(px(150.)).h(px(210.)).rounded_md().bg(rgb(0x282135))
                    .flex().items_center().justify_center().text_color(rgb(ACCENT)).child("✦")
            };
            let mut details=div().w(px(266.)).h_full().p_4().bg(rgb(PANEL)).border_l_1().border_color(rgb(BORDER))
                .flex().flex_col().gap_3()
                .child(div().flex().justify_between()
                    .child(div().text_sm().text_color(rgb(MUTED)).child("DETALHES"))
                    .child(self.button("close","×",false).on_click(cx.listener(|s,_,_,cx|{s.selected=None;cx.notify();}))))
                .child(preview)
                .child(div().w_full().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(TEXT)).child(book.title.clone()))
                .child(div().text_sm().text_color(rgb(MUTED)).child(book.author.clone()))

                .child(div().text_xs().text_color(rgb(MUTED)).child(format!("{} · {}",book.format, book.status)))
                .child(div().text_xs().text_color(rgb(MUTED)).child(format!("Modificado há {} dias", std::time::SystemTime::now().duration_since(book.modified).map(|d|d.as_secs()/86400).unwrap_or(0))))
                .child(self.button("open","Abrir livro ↗",true).on_click(move |_,_,cx|{cx.open_with_system(&path);}))
                .child(self.button("fav",if book.favorite {"♥  Remover favorito"} else {"♡  Adicionar favorito"},false)
                    .on_click(cx.listener(move |s,_,_,cx|{s.toggle_favorite(&path2);cx.notify();})))
                .child(self.button("state",format!("Estado: {}  ↻",book.status),false)
                    .on_click(cx.listener(move |s,_,_,cx|{s.change_status(&path3);cx.notify();})))
                .child(div().mt_4().text_xs().text_color(rgb(MUTED)).child("CATEGORIAS · CLICA PARA ASSOCIAR"));
            for (i,cat) in self.categories.iter().enumerate() {
                let has=book.categories.contains(cat);
                let p=book.path.clone();
                let name=cat.clone();
                details=details.child(self.button(&format!("assign-{i}"),format!("{} {}",if has {"☑"} else {"□"},cat),has)
                    .on_click(cx.listener(move |s,_,_,cx|{s.toggle_assignment(&p,&name);cx.notify();})));
            }
            if let View::Category(cat)=&self.view {
                let old=cat.clone(); let old2=old.clone();
                details=details.child(div().mt_5().text_xs().text_color(rgb(MUTED)).child("GESTÃO DE COLEÇÃO"))
                    .child(self.button("rename-category","Renomear coleção",false)
                        .on_click(cx.listener(move |s,_,_,cx|{s.rename_category(&old);cx.notify();})))
                    .child(self.button("remove-category","Eliminar coleção",false)
                        .on_click(cx.listener(move |s,_,_,cx|{s.remove_category(&old2);cx.notify();})));
            }
            details.child(div().flex_1()).child(div().w_full().overflow_hidden().text_xs().text_color(rgb(MUTED)).child(shorten(&book.path.display().to_string(), 90)))
        } else {
            div().w(px(0.)).h_full()
        };

        div().size_full().flex().bg(rgb(BG)).text_color(rgb(TEXT))
            .child(side).child(main_panel).child(right)
    }
}

fn shorten(s: &str, max: usize) -> String {
    let count=s.chars().count();
    if count<=max {return s.to_string();}
    format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
}

fn main() {
    gpui_platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(|cx: &mut App| {
            cx.spawn(async move |cx| {
                cx.update(|cx| {
                    match cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Veloria::new())) {
                        Ok(_) => cx.activate(true),
                        Err(error) => eprintln!("Veloria: erro ao abrir janela: {error:?}"),
                    }
                })
            }).detach();
        });
}
