use std::{io, time::Duration, thread};

use nu_protocol::{PipelineData, Value, Span};
use tui::{Terminal, backend::CrosstermBackend, layout::{Direction, Rect}, widgets::{Clear, Widget}, text::Text, buffer::Buffer};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error};
use tui::{
    backend::{Backend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[derive(Default)]
struct Label<'a> {
    text: &'a str,
}

impl<'a> Widget for Label<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.left(), area.top(), self.text, Style::default());
    }
}

impl<'a> Label<'a> {
    fn text(mut self, text: &'a str) -> Label<'a> {
        self.text = text;
        self
    }
}

struct App<'a> {
    state: TableState,
    header: Row<'a>,
    items: Vec<Row<'a>>,
    values: Vec<Value>,
    current_row: Option<String>,
}

impl<'a> App<'a> {
    fn new(header: Row<'a>, rows: Vec<Row<'a>>, values: Vec<Value>) -> App<'a> {
        App {
            state: TableState::default(),
            header,
            items: rows,
            values,
            current_row: None,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn select_current(&mut self) {
        if let Some(row) = self.state.selected() {
            self.current_row = match &self.values[row].as_string() {
                Ok(data) => Some(data.clone()),
                Err(e) => Some(e.to_string()),
            }
        }
    }
}

fn main_(header: Row, rows: Vec<Row>, data: PipelineData) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let list = if let PipelineData::Value(Value::List { vals, .. }, _) = data {
        vals
    } else {
         vec![Value::Nothing { span: Span::new(0, 0) }] 
    };

    // create app and run it
    let app = App::new(header, rows, list);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') if app.current_row.is_none() => return Ok(()),
                KeyCode::Char('q') if app.current_row.is_some() => app.current_row = None,
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                KeyCode::Enter => app.select_current(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(5)
        .split(f.size());

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::DarkGray);
    let header_cells = app.header.clone().style(Style::default().fg(Color::Red));
        // .iter()
        // .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = header_cells
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
    // let rows = app.items.iter().map(|item| {

        // // let height = item
            // // .
            // // .map(|content| content.chars().filter(|c| *c == '\n').count())
            // // .max()
            // // .unwrap_or(0)
            // // + 1;
        // // let cells = item.iter().map(|c| Cell::from(*c));
        // // Row::new(cells).height(height as u16).bottom_margin(1)
    // });
    let t = Table::new(app.items.clone())
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Table"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);
    f.render_stateful_widget(t, rects[0], &mut app.state);

    if let Some(row) = &app.current_row {
        let block = Block::default().title("Popup").borders(Borders::ALL);
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);
        f.render_widget(Label { text: row }, area);
    }
}
pub struct NuSpace {
    pub terminal: Terminal<tui::backend::CrosstermBackend<std::io::Stdout>>,
}

impl NuSpace {
    pub fn write(&mut self, data: PipelineData) -> Result<(), io::Error> {

        let error_style = Style::default().bg(Color::Red);
         // setup terminal

        let mut rows = vec![ Row::new(vec!["Null"]) ];
        let mut header = Row::new(vec!["Null"]);

        if let PipelineData::Value(val, _) = &data {
            match val {
                Value::List { vals: list_values, .. } => {
                    if let Value::Record { cols, .. } = &list_values[0] {
                        header = Row::new(cols.to_owned());
                        rows = list_values.into_iter().map(|value| {
                            if let Value::Record { vals, .. } = value {
                                Row::new(vals.into_iter().map(|v| v.as_string().unwrap())).style(Style::default())
                            }
                            else
                            {
                                Row::new(vec!["Not a Record.."]).style(error_style)
                            }
                        }).collect();
                    }
                },
                Value::Record { cols, vals, .. } => {
                    header = Row::new(cols.into_iter().map(|i| { return i.to_string() })).style(Style::default());
                    rows = vec![
                        Row::new(vals.into_iter().map(|v| v.as_string().unwrap())).style(Style::default())
                    ]
                },
                _ => {
                    rows = vec![Row::new(vec!["foo"])]
                }
            }
        }

        main_(header, rows, data).unwrap();

        // enable_raw_mode()?;
        // let mut stdout = io::stdout();
        // execute!(stdout, Clear(ClearType::All))?;
        // execute!(stdout, cursor::MoveTo(5,5));
        // execute!(stdout, EnterAlternateScreen, Clear(ClearType::All), EnableMouseCapture)?;

          // stdout.flush()?;

        // self.terminal.draw(|f| {
            // let size = f.size();
            // // let list = Table::new(rows).header(header);
            // let list = List::new(vec![
                // ListItem::new("Foo".to_string()),
                // ListItem::new("Foo".to_string()),
                // ListItem::new("Foo".to_string()),
                // ListItem::new("Foo".to_string()),
                // ListItem::new("Foo".to_string()),
            // ]);
            // f.render_widget(list, size);
        // })?;
        // terminal.draw(|f| {
            // let size = f.size();
            // let block = Block::default()
                // .title("Block")
                // .borders(Borders::ALL);
            // f.render_widget(block, size);
        // })?;

        // thread::sleep(Duration::from_millis(5000));

        // restore terminal
        // disable_raw_mode()?;
        // execute!(
            // stdout,
            // self.terminal.backend_mut(),
            // LeaveAlternateScreen,
            // DisableMouseCapture
        // )?;
        // self.terminal.show_cursor()?;

        Ok(())
    }

    pub fn new() -> Result<Self, io::Error> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        Ok (
            NuSpace {
                terminal: Terminal::new(backend)?
            }
        )
    }
}
