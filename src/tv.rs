use std::{
    io,
    process::Command as ProcessCommand,
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::config::{Theme, ThemeBorder, ThemeColor};

const CONTENT_MAX_WIDTH: u16 = 72;
const BUTTON_HEIGHT: u16 = 4;
const RETRY_WINDOW: Duration = Duration::from_secs(60);
const RETRY_DELAY: Duration = Duration::from_secs(2);
const RICKROLL_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const INPUT: usize = 0;
const WATCH: usize = 1;
const RICKROLL: usize = 2;
const PAUSE: usize = 3;
const PLAY: usize = 4;
const STOP: usize = 5;
const BACK: usize = 6;
const LAST_SELECTION: usize = BACK;

#[derive(Clone, Debug)]
pub enum TvCommand {
    Cast(String),
    Rickroll,
    Pause,
    Play,
    Stop,
}

impl TvCommand {
    pub fn busy_message(&self) -> &'static str {
        match self {
            Self::Cast(_) => "Avataan video televisiossa…",
            Self::Rickroll => "Valmistellaan rickrollia… :D",
            Self::Pause => "Laitetaan video tauolle…",
            Self::Play => "Jatketaan videota…",
            Self::Stop => "Lopetetaan toisto…",
        }
    }

    fn success_message(&self) -> &'static str {
        match self {
            Self::Cast(_) | Self::Rickroll => "Video toistuu televisiossa.",
            Self::Pause => "Video on tauolla.",
            Self::Play => "Video jatkuu.",
            Self::Stop => "Toisto lopetettu.",
        }
    }

    fn catt_args(&self) -> Vec<String> {
        match self {
            Self::Cast(url) => vec!["cast".to_owned(), url.clone()],
            Self::Rickroll => vec!["cast".to_owned(), RICKROLL_URL.to_owned()],
            Self::Pause => vec!["pause".to_owned()],
            Self::Play => vec!["play".to_owned()],
            Self::Stop => vec!["stop".to_owned()],
        }
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub command: TvCommand,
    pub result: Result<(), String>,
}

pub enum Intent {
    None,
    Back,
    Run(TvCommand),
}

pub struct State {
    input: String,
    selected: usize,
    input_area: Rect,
    button_areas: Vec<Rect>,
    status: String,
    busy: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            input: String::new(),
            selected: INPUT,
            input_area: Rect::default(),
            button_areas: Vec::new(),
            status: "Liitä YouTube-linkki tai kokeile RICKROLL :D".to_owned(),
            busy: None,
        }
    }
}

impl State {
    pub fn reset(&mut self) {
        self.selected = INPUT;
        self.status = "Liitä YouTube-linkki tai kokeile RICKROLL :D".to_owned();
        self.busy = None;
    }

    pub fn begin(&mut self, command: &TvCommand) {
        self.busy = Some(command.busy_message().to_owned());
    }

    pub fn finish(&mut self, outcome: Outcome) {
        self.busy = None;
        match outcome.result {
            Ok(()) => self.status = outcome.command.success_message().to_owned(),
            Err(error) => {
                eprintln!("momarchy: Chromecast command failed: {error}");
                self.status = "Televisiota ei saatu yhteyteen. Yritä uudelleen.".to_owned();
            }
        }
    }

    pub fn finish_dry_run(&mut self, command: TvCommand) {
        self.busy = None;
        self.status = format!("DEVELOPMENT MODE — Chromecast: {command:?}");
    }

    pub fn render(&mut self, frame: &mut Frame, frame_area: Rect, theme: &Theme, live: bool) {
        frame.render_widget(Block::default().style(base_style(theme)), frame_area);

        if let Some(message) = self.busy.as_deref() {
            self.input_area = Rect::default();
            self.button_areas.clear();
            render_busy(frame, frame_area, theme, message);
            return;
        }

        let area = centered_max_width(inset(frame_area, theme.layout.margin), CONTENT_MAX_WIDTH);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(5),
                Constraint::Length(2),
                Constraint::Min(12),
                Constraint::Length(3),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new("KATSO TELEVISIOSTA")
                .alignment(Alignment::Center)
                .style(base_style(theme).add_modifier(Modifier::BOLD)),
            rows[0],
        );
        frame.render_widget(
            Paragraph::new("Liitä YouTube-linkki ja paina KATSO.")
                .alignment(Alignment::Center)
                .style(muted_style(theme)),
            rows[1],
        );

        self.input_area = rows[2];
        let input_selected = self.selected == INPUT;
        let input_style = if input_selected {
            selected_style(theme).add_modifier(Modifier::BOLD)
        } else {
            base_style(theme)
        };
        let input_text = if self.input.is_empty() {
            "Liitä linkki tähän…"
        } else {
            self.input.as_str()
        };
        frame.render_widget(
            Paragraph::new(input_text)
                .block(
                    Block::default()
                        .title(" YouTube-linkki ")
                        .borders(Borders::ALL)
                        .border_type(border_type(theme.border))
                        .style(input_style),
                )
                .style(if self.input.is_empty() && !input_selected {
                    muted_style(theme)
                } else {
                    input_style
                })
                .wrap(Wrap { trim: false }),
            self.input_area,
        );

        frame.render_widget(
            Paragraph::new("Liitä normaalisti terminaalin paste-komennolla. Enter aloittaa.")
                .alignment(Alignment::Center)
                .style(muted_style(theme)),
            rows[3],
        );

        let button_specs = [
            ("KATSO", "Toista linkki"),
            ("RICKROLL :D", "Nopea testi"),
            ("TAUKO", "Pysäytä hetkeksi"),
            ("JATKA", "Jatka toistoa"),
            ("LOPETA", "Lopeta televisiosta"),
            ("TAKAISIN", "Palaa alkuun"),
        ];
        self.button_areas = grid_areas(rows[4], button_specs.len(), 2, theme.layout.gap);
        for (index, area) in self.button_areas.iter().copied().enumerate() {
            let selection = index + 1;
            let style = if self.selected == selection {
                selected_style(theme).add_modifier(Modifier::BOLD)
            } else {
                base_style(theme)
            };
            let (label, hint) = button_specs[index];
            frame.render_widget(
                Paragraph::new(format!("{label}\n{hint}"))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(border_type(theme.border))
                            .style(style),
                    )
                    .style(style)
                    .wrap(Wrap { trim: true }),
                area,
            );
        }

        let mode = if live {
            ""
        } else {
            "DEVELOPMENT MODE — televisiota ei ohjata\n"
        };
        frame.render_widget(
            Paragraph::new(format!("{mode}{}", self.status))
                .alignment(Alignment::Center)
                .style(muted_style(theme))
                .wrap(Wrap { trim: true }),
            rows[5],
        );
    }

    pub fn handle_event(&mut self, event: Event) -> Intent {
        if self.busy.is_some() {
            return Intent::None;
        }

        match event {
            Event::Paste(text) => {
                self.set_input(&text);
                self.selected = INPUT;
                Intent::None
            }
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => Intent::Back,
                KeyCode::Tab => {
                    self.selected = (self.selected + 1) % (LAST_SELECTION + 1);
                    Intent::None
                }
                KeyCode::BackTab => {
                    self.selected = if self.selected == 0 {
                        LAST_SELECTION
                    } else {
                        self.selected - 1
                    };
                    Intent::None
                }
                KeyCode::Down => {
                    self.move_down();
                    Intent::None
                }
                KeyCode::Up => {
                    self.move_up();
                    Intent::None
                }
                KeyCode::Left => {
                    self.move_left();
                    Intent::None
                }
                KeyCode::Right => {
                    self.move_right();
                    Intent::None
                }
                KeyCode::Backspace if self.selected == INPUT => {
                    self.input.pop();
                    Intent::None
                }
                KeyCode::Enter => self.activate_selected(),
                KeyCode::Char(character)
                    if self.selected == INPUT
                        && !key.modifiers.intersects(
                            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                        ) =>
                {
                    if self.input.len() < 2048 {
                        self.input.push(character);
                    }
                    Intent::None
                }
                _ => Intent::None,
            },
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                if contains(self.input_area, mouse.column, mouse.row) {
                    self.selected = INPUT;
                    return Intent::None;
                }
                if let Some(index) = self
                    .button_areas
                    .iter()
                    .position(|area| contains(*area, mouse.column, mouse.row))
                {
                    self.selected = index + 1;
                    return self.activate_selected();
                }
                Intent::None
            }
            _ => Intent::None,
        }
    }

    fn set_input(&mut self, text: &str) {
        let line = text.lines().next().unwrap_or("").trim();
        self.input = line.chars().take(2048).collect();
        self.status = "Linkki liitetty. Paina KATSO.".to_owned();
    }

    fn activate_selected(&mut self) -> Intent {
        match self.selected {
            INPUT | WATCH => {
                let url = self.input.trim();
                if !is_youtube_url(url) {
                    self.status = "Liitä kelvollinen YouTube-linkki.".to_owned();
                    self.selected = INPUT;
                    Intent::None
                } else {
                    Intent::Run(TvCommand::Cast(url.to_owned()))
                }
            }
            RICKROLL => Intent::Run(TvCommand::Rickroll),
            PAUSE => Intent::Run(TvCommand::Pause),
            PLAY => Intent::Run(TvCommand::Play),
            STOP => Intent::Run(TvCommand::Stop),
            BACK => Intent::Back,
            _ => Intent::None,
        }
    }

    fn move_down(&mut self) {
        self.selected = match self.selected {
            INPUT => WATCH,
            WATCH | RICKROLL => PAUSE + (self.selected - WATCH),
            PAUSE | PLAY => STOP + (self.selected - PAUSE),
            STOP | BACK => self.selected,
            _ => INPUT,
        };
    }

    fn move_up(&mut self) {
        self.selected = match self.selected {
            INPUT => INPUT,
            WATCH | RICKROLL => INPUT,
            PAUSE | PLAY => WATCH + (self.selected - PAUSE),
            STOP | BACK => PAUSE + (self.selected - STOP),
            _ => INPUT,
        };
    }

    fn move_left(&mut self) {
        if matches!(self.selected, RICKROLL | PLAY | BACK) {
            self.selected -= 1;
        }
    }

    fn move_right(&mut self) {
        if matches!(self.selected, WATCH | PAUSE | STOP) {
            self.selected += 1;
        }
    }
}

pub fn execute(command: TvCommand) -> Outcome {
    let result = run_catt_with_retry(&command.catt_args());
    Outcome { command, result }
}

fn run_catt_with_retry(args: &[String]) -> Result<(), String> {
    let started = Instant::now();

    loop {
        let last_error = match ProcessCommand::new("catt").args(args).output() {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !stderr.is_empty() {
                    stderr
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    format!("catt exited with {}", output.status)
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err("catt-ohjelmaa ei löydy; suorita Momarchy provisioning".to_owned());
            }
            Err(error) => error.to_string(),
        };

        if started.elapsed() >= RETRY_WINDOW {
            return Err(format!(
                "Chromecast command failed after retries: {last_error}"
            ));
        }
        thread::sleep(RETRY_DELAY);
    }
}

fn is_youtube_url(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or("");
    matches!(
        host,
        "youtube.com"
            | "www.youtube.com"
            | "m.youtube.com"
            | "music.youtube.com"
            | "youtu.be"
            | "www.youtu.be"
    )
}

fn render_busy(frame: &mut Frame, area: Rect, theme: &Theme, message: &str) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Length(7),
            Constraint::Percentage(38),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(format!(
            "{message}\n\nEtsitään televisiota tarvittaessa uudelleen.\nOdota hetki."
        ))
        .alignment(Alignment::Center)
        .style(base_style(theme).add_modifier(Modifier::BOLD))
        .wrap(Wrap { trim: true }),
        rows[1],
    );
}

fn base_style(theme: &Theme) -> Style {
    Style::default()
        .fg(theme_color(theme.colors.text))
        .bg(theme_color(theme.colors.background))
}

fn muted_style(theme: &Theme) -> Style {
    Style::default()
        .fg(theme_color(theme.colors.muted))
        .bg(theme_color(theme.colors.background))
}

fn selected_style(theme: &Theme) -> Style {
    Style::default()
        .fg(theme_color(theme.colors.selected_text))
        .bg(theme_color(theme.colors.selected_background))
}

fn theme_color(color: ThemeColor) -> Color {
    match color {
        ThemeColor::Black => Color::Black,
        ThemeColor::Red => Color::Red,
        ThemeColor::Green => Color::Green,
        ThemeColor::Yellow => Color::Yellow,
        ThemeColor::Blue => Color::Blue,
        ThemeColor::Magenta => Color::Magenta,
        ThemeColor::Cyan => Color::Cyan,
        ThemeColor::Gray => Color::Gray,
        ThemeColor::DarkGray => Color::DarkGray,
        ThemeColor::White => Color::White,
    }
}

fn border_type(border: ThemeBorder) -> BorderType {
    match border {
        ThemeBorder::Plain => BorderType::Plain,
        ThemeBorder::Rounded => BorderType::Rounded,
        ThemeBorder::Double => BorderType::Double,
        ThemeBorder::Thick => BorderType::Thick,
    }
}

fn inset(area: Rect, amount: u16) -> Rect {
    let horizontal = amount.min(area.width / 2);
    let vertical = amount.min(area.height / 2);
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn centered_max_width(area: Rect, max_width: u16) -> Rect {
    let width = area.width.min(max_width);
    Rect {
        x: area.x.saturating_add(area.width.saturating_sub(width) / 2),
        y: area.y,
        width,
        height: area.height,
    }
}

fn grid_areas(area: Rect, count: usize, columns: u16, gap: u16) -> Vec<Rect> {
    let rows = count.div_ceil(usize::from(columns)) as u16;
    let natural_height = rows
        .saturating_mul(BUTTON_HEIGHT)
        .saturating_add(rows.saturating_sub(1).saturating_mul(gap));
    let top = area
        .y
        .saturating_add(area.height.saturating_sub(natural_height) / 2);
    let total_horizontal_gap = gap.saturating_mul(columns.saturating_sub(1));
    let usable_width = area.width.saturating_sub(total_horizontal_gap);
    let column_width = usable_width / columns;
    let mut areas = Vec::with_capacity(count);

    for index in 0..count {
        let index = index as u16;
        let row = index / columns;
        let column = index % columns;
        let x = area.x.saturating_add(
            column.saturating_mul(column_width.saturating_add(gap)),
        );
        let width = if column + 1 == columns {
            area.x.saturating_add(area.width).saturating_sub(x)
        } else {
            column_width
        };
        let y = top.saturating_add(row.saturating_mul(BUTTON_HEIGHT.saturating_add(gap)));
        areas.push(Rect::new(x, y, width, BUTTON_HEIGHT.min(area.height)));
    }
    areas
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_youtube_urls() {
        assert!(is_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_youtube_url("https://m.youtube.com/shorts/example"));
    }

    #[test]
    fn rejects_non_youtube_input() {
        assert!(!is_youtube_url("not a url"));
        assert!(!is_youtube_url("https://example.com/watch?v=dQw4w9WgXcQ"));
    }
}
