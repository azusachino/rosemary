use crate::{
    embed::EmbeddingProvider,
    recall::{recall_data, RecallData},
    vector::VectorStore,
};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use libsql::Connection;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;

pub struct App {
    pub input: String,
    pub results_data: Option<RecallData>,
    pub selected_index: usize,
    pub preview_content: String,
    pub list_state: ListState,
    pub recall_results_count: usize,
    // We need to store titles and paths for the UI
    pub display_items: Vec<(String, String, String)>, // (topic_id, title, score)
}

impl App {
    pub fn new() -> App {
        App {
            input: String::new(),
            results_data: None,
            selected_index: 0,
            preview_content: String::new(),
            list_state: ListState::default(),
            recall_results_count: 0,
            display_items: Vec::new(),
        }
    }

    pub fn next(&mut self) {
        if self.recall_results_count == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.recall_results_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_index = i;
    }

    pub fn previous(&mut self) {
        if self.recall_results_count == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.recall_results_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected_index = i;
    }
}

pub async fn run_tui(
    conn: Connection,
    store: VectorStore,
    embedder: Arc<dyn EmbeddingProvider>,
) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app, conn, store, embedder).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor().map_err(|e| anyhow::anyhow!("{:?}", e))?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    conn: Connection,
    store: VectorStore,
    embedder: Arc<dyn EmbeddingProvider>,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app)).map_err(|e| anyhow::anyhow!("{:?}", e))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        app.input.push(c);
                        // Trigger search
                        if let Ok(data) = recall_data(&app.input, &conn, &store, embedder.as_ref(), 10).await {
                            let ranked = data.ranked(10);
                            app.recall_results_count = ranked.len();
                            app.display_items = ranked.iter().map(|r| (r.topic_id.to_string(), r.title.to_string(), format!("{:.2}", r.score))).collect();
                            app.results_data = Some(data);
                            app.list_state.select(Some(0));
                            app.selected_index = 0;
                            update_preview(&mut app);
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        // Trigger search
                        if app.input.is_empty() {
                            app.results_data = None;
                            app.display_items.clear();
                            app.recall_results_count = 0;
                            app.preview_content.clear();
                        } else {
                            if let Ok(data) = recall_data(&app.input, &conn, &store, embedder.as_ref(), 10).await {
                                let ranked = data.ranked(10);
                                app.recall_results_count = ranked.len();
                                app.display_items = ranked.iter().map(|r| (r.topic_id.to_string(), r.title.to_string(), format!("{:.2}", r.score))).collect();
                                app.results_data = Some(data);
                                app.list_state.select(Some(0));
                                app.selected_index = 0;
                                update_preview(&mut app);
                            }
                        }
                    }
                    KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Down => {
                        app.next();
                        update_preview(&mut app);
                    }
                    KeyCode::Up => {
                        app.previous();
                        update_preview(&mut app);
                    }
                    KeyCode::Enter => {
                        if let Some(data) = &app.results_data {
                            let ranked = data.ranked(10);
                            if let Some(selected) = ranked.get(app.selected_index) {
                                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                                // We need to restore terminal before launching editor
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                terminal.show_cursor().map_err(|e| anyhow::anyhow!("{:?}", e))?;

                                std::process::Command::new(editor)
                                    .arg(selected.file_path)
                                    .status()?;

                                // Re-setup terminal after editor exits
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                terminal.clear().map_err(|e| anyhow::anyhow!("{:?}", e))?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn update_preview(app: &mut App) {
    if let Some(data) = &app.results_data {
        let ranked = data.ranked(10);
        if let Some(selected) = ranked.get(app.selected_index) {
            // Load file content
            if let Ok(content) = std::fs::read_to_string(selected.file_path) {
                app.preview_content = content;
            } else {
                app.preview_content = format!("Error reading file: {}", selected.file_path);
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Search Rosemary"));
    f.render_widget(input, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    let items: Vec<ListItem> = app
        .display_items
        .iter()
        .map(|(_, title, score)| {
            ListItem::new(format!("[{}] {}", score, title))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, body_chunks[0], &mut app.list_state);

    let preview = Paragraph::new(app.preview_content.as_str())
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(preview, body_chunks[1]);

    let footer = Paragraph::new("Esc: Quit | Up/Down: Navigate | Enter: Open Editor")
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_navigation() {
        let mut app = App::new();
        app.display_items = vec![
            ("id1".into(), "Title 1".into(), "0.9".into()),
            ("id2".into(), "Title 2".into(), "0.8".into()),
        ];
        app.recall_results_count = 2;
        app.list_state.select(Some(0));
        app.selected_index = 0;

        // Start at 0
        assert_eq!(app.selected_index, 0);

        // Next -> 1
        app.next();
        assert_eq!(app.selected_index, 1);
        assert_eq!(app.list_state.selected(), Some(1));

        // Next -> 0 (wrap)
        app.next();
        assert_eq!(app.selected_index, 0);

        // Previous -> 1 (wrap)
        app.previous();
        assert_eq!(app.selected_index, 1);

        // Previous -> 0
        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_app_input() {
        let mut app = App::new();
        app.input.push('r');
        app.input.push('u');
        app.input.push('s');
        app.input.push('t');
        assert_eq!(app.input, "rust");

        app.input.pop();
        assert_eq!(app.input, "rus");
    }
}
