use std::io::{self, stdout};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

const BUTTONS: [&str; 6] = [
    "Internet",
    "Sähköposti",
    "Kuvat",
    "YouTube",
    "Kysy mitä vain",
    "Apua",
];

pub fn run() -> io::Result<()> {
    execute!(stdout(), EnableMouseCapture)?;
    let result = ratatui::run(|terminal| App::default().run(terminal));
    let _ = execute!(stdout(), DisableMouseCapture);
    result
}

struct App {
    selected: usize,
    button_areas: [Rect; BUTTONS.len()],
    message: &'static str,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selected: 0,
            button_areas: [Rect::default(); BUTTONS.len()],
            message: "Valitse hiirellä tai nuolinäppäimillä.",
            exit: false,
        }
    }
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event(event::read()?);
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(15),
                Constraint::Length(2),
            ])
            .split(frame.area());

        let title = Paragraph::new("Momarchy\nTervetuloa")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(title, areas[0]);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(areas[1]);

        for (row_index, row) in rows.iter().enumerate() {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*row);

            for (column_index, area) in columns.iter().enumerate() {
                let index = row_index * 2 + column_index;
                self.button_areas[index] = *area;

                let style = if index == self.selected {
                    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default()
                };

                let block = Block::default().borders(Borders::ALL).style(style);
                let button = Paragraph::new(BUTTONS[index])
                    .alignment(Alignment::Center)
                    .block(block)
                    .style(style);
                frame.render_widget(button, *area);
            }
        }

        let footer = Paragraph::new(self.message).alignment(Alignment::Center);
        frame.render_widget(footer, areas[2]);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.exit = true,
                KeyCode::Left if self.selected % 2 == 1 => self.selected -= 1,
                KeyCode::Right if self.selected % 2 == 0 => self.selected += 1,
                KeyCode::Up if self.selected >= 2 => self.selected -= 2,
                KeyCode::Down if self.selected < 4 => self.selected += 2,
                KeyCode::Enter => self.activate(),
                _ => {}
            },
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                if let Some(index) = self
                    .button_areas
                    .iter()
                    .position(|area| contains(*area, mouse.column, mouse.row))
                {
                    self.selected = index;
                    self.activate();
                }
            }
            _ => {}
        }
    }

    fn activate(&mut self) {
        self.message = match self.selected {
            0 => "Internetin käynnistys tulee seuraavaksi.",
            1 => "Sähköpostin käynnistys tulee seuraavaksi.",
            2 => "Kuvat tulevat seuraavaksi.",
            3 => "YouTuben käynnistys tulee seuraavaksi.",
            4 => "Kysy mitä vain tulee seuraavaksi.",
            5 => "Apunäkymä tulee seuraavaksi.",
            _ => "",
        };
    }
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}
