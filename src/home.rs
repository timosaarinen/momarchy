use std::{
    io::{self, BufRead, Write, stdout},
    path::Path,
    process::Command,
    sync::mpsc::{self, Sender},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::{
    config::{self, Action, Button, Config, Theme, ThemeBorder, ThemeColor},
    watch::{self, WatchEvent},
};

const MENU_MAX_WIDTH: u16 = 64;
const BUTTON_HEIGHT: u16 = 4;

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub automation: bool,
    pub live_actions: bool,
}

pub fn run(options: Options) -> io::Result<()> {
    let initial = config::initialize()?;
    let mut app = App::new(
        initial.config,
        options.live_actions,
        initial.used_embedded_fallback,
    );

    if options.automation {
        return run_automation(&mut app, &initial.path);
    }

    let _mouse = MouseCaptureGuard::enable()?;
    ratatui::run(|terminal| app.run(terminal, &initial.path))
}

struct MouseCaptureGuard;

impl MouseCaptureGuard {
    fn enable() -> io::Result<Self> {
        execute!(stdout(), EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for MouseCaptureGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), DisableMouseCapture);
    }
}

enum RuntimeEvent {
    Terminal(Event),
    TerminalFailed(String),
    ConfigChanged,
    WatchFailed,
}

struct App {
    config: Config,
    screen: String,
    selected: usize,
    button_areas: Vec<Rect>,
    status: String,
    exit: bool,
    live_actions: bool,
}

impl App {
    fn new(config: Config, live_actions: bool, used_embedded_fallback: bool) -> Self {
        let screen = config.home.clone();
        let status = if used_embedded_fallback {
            "Asetuksissa on virhe. Käytetään turvallisia oletusasetuksia.".to_owned()
        } else {
            "Valitse hiirellä tai nuolinäppäimillä.".to_owned()
        };

        Self {
            config,
            screen,
            selected: 0,
            button_areas: Vec::new(),
            status,
            exit: false,
            live_actions,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal, config_path: &Path) -> io::Result<()> {
        let (sender, receiver) = mpsc::channel();
        spawn_terminal_reader(sender.clone())?;

        let config_dir = config_path
            .parent()
            .ok_or_else(|| io::Error::other("Momarchy config path has no parent directory"))?;
        let watch_sender = sender.clone();
        watch::spawn(config_dir, move |event| {
            let event = match event {
                WatchEvent::ConfigChanged => RuntimeEvent::ConfigChanged,
                WatchEvent::Failed(error) => {
                    eprintln!("momarchy: config watcher failed: {error}");
                    RuntimeEvent::WatchFailed
                }
            };
            let _ = watch_sender.send(event);
        })?;

        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;

            match receiver.recv() {
                Ok(RuntimeEvent::Terminal(event)) => self.handle_event(event)?,
                Ok(RuntimeEvent::TerminalFailed(error)) => {
                    return Err(io::Error::other(format!("terminal input failed: {error}")));
                }
                Ok(RuntimeEvent::ConfigChanged) => self.reload_config(config_path),
                Ok(RuntimeEvent::WatchFailed) => {
                    self.status = "Asetusten automaattinen päivitys ei toimi.".to_owned();
                }
                Err(_) => return Err(io::Error::other("Momarchy event sources stopped")),
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let theme = self.config.theme.clone();
        let frame_area = frame.area();
        frame.render_widget(Block::default().style(base_style(&theme)), frame_area);

        let area = inset(frame_area, theme.layout.margin);
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(16),
                Constraint::Length(4),
            ])
            .split(area);

        let (title, subtitle, body) = {
            let screen = self.current_screen();
            (
                screen.title.clone(),
                screen.subtitle.clone(),
                screen.body.clone(),
            )
        };

        let header_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(areas[0]);
        let title = Paragraph::new(title)
            .alignment(Alignment::Center)
            .style(base_style(&theme).add_modifier(Modifier::BOLD));
        let subtitle = Paragraph::new(subtitle)
            .alignment(Alignment::Center)
            .style(muted_style(&theme));
        frame.render_widget(title, header_rows[0]);
        frame.render_widget(subtitle, header_rows[1]);

        if let Some(body) = body {
            let content_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(6)])
                .split(areas[1]);

            let body = Paragraph::new(body)
                .alignment(Alignment::Center)
                .style(base_style(&theme))
                .wrap(Wrap { trim: true });
            frame.render_widget(body, content_areas[0]);
            self.render_buttons(frame, content_areas[1], &theme);
        } else {
            self.render_buttons(frame, areas[1], &theme);
        }

        let mode = if self.live_actions {
            ""
        } else {
            "DEVELOPMENT MODE — external programs are not launched\n"
        };
        let footer = Paragraph::new(format!("{mode}{}", self.status))
            .alignment(Alignment::Center)
            .style(muted_style(&theme))
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, areas[2]);
    }

    fn render_buttons(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let selected = self.selected;
        let buttons = &self
            .config
            .screen(&self.screen)
            .expect("current screen must exist in validated config")
            .buttons;
        let button_areas = &mut self.button_areas;
        let menu_area = centered_max_width(area, MENU_MAX_WIDTH);

        *button_areas = grid_areas(
            menu_area,
            buttons.len(),
            theme.layout.columns,
            theme.layout.gap,
        );

        for (index, button_area) in button_areas.iter().copied().enumerate() {
            let button = &buttons[index];
            let is_selected = index == selected;
            let style = if is_selected {
                selected_style(theme).add_modifier(Modifier::BOLD)
            } else {
                base_style(theme)
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(border_type(theme.border))
                .style(style);
            let text = format!("{}\n{}", button.label, button.hint);
            let widget = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(block)
                .style(style)
                .wrap(Wrap { trim: true });
            frame.render_widget(widget, button_area);
        }
    }

    fn current_screen(&self) -> &config::Screen {
        self.config
            .screen(&self.screen)
            .expect("current screen must exist in validated config")
    }

    fn buttons(&self) -> &[Button] {
        &self.current_screen().buttons
    }

    fn columns(&self) -> usize {
        usize::from(self.config.theme.layout.columns)
    }

    fn handle_event(&mut self, event: Event) -> io::Result<()> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => self.back_or_exit(),
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Left => self.move_left(),
                KeyCode::Right => self.move_right(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Enter => self.activate()?,
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
                    self.activate()?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn move_left(&mut self) {
        let columns = self.columns();
        if self.selected % columns != 0 {
            self.selected -= 1;
        }
    }

    fn move_right(&mut self) {
        let columns = self.columns();
        if self.selected % columns + 1 < columns && self.selected + 1 < self.buttons().len() {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        let columns = self.columns();
        if self.selected >= columns {
            self.selected -= columns;
        }
    }

    fn move_down(&mut self) {
        let columns = self.columns();
        if self.selected + columns < self.buttons().len() {
            self.selected += columns;
        }
    }

    fn back_or_exit(&mut self) {
        if self.screen == self.config.home {
            self.exit = true;
        } else {
            let home = self.config.home.clone();
            self.go_to(home);
        }
    }

    fn go_to(&mut self, screen: String) {
        self.screen = screen;
        self.selected = 0;
        self.status = self.current_screen().subtitle.clone();
    }

    fn activate(&mut self) -> io::Result<()> {
        let action = self.buttons()[self.selected].action.clone();
        match action {
            Action::Navigate(screen) => self.go_to(screen),
            Action::Message(message) => self.status = message,
            Action::Open {
                target,
                live_message,
            } => {
                if self.live_actions {
                    crate::browser::open(&target)?;
                    self.status = live_message;
                } else {
                    self.status = format!("DEVELOPMENT MODE — browser: {target}");
                }
            }
            Action::Command {
                kind,
                program,
                args,
                live_message,
            } => {
                if self.live_actions {
                    Command::new(&program).args(&args).spawn()?;
                    self.status = live_message;
                } else {
                    self.status =
                        format!("DEVELOPMENT MODE — {kind}: {program} {}", args.join(" "));
                }
            }
        }
        Ok(())
    }

    fn select_id(&mut self, id: &str) -> bool {
        if let Some(index) = self.buttons().iter().position(|button| button.id == id) {
            self.selected = index;
            true
        } else {
            false
        }
    }

    fn automation_key(&mut self, key: &str) -> io::Result<()> {
        match key {
            "left" => self.move_left(),
            "right" => self.move_right(),
            "up" => self.move_up(),
            "down" => self.move_down(),
            "enter" => self.activate()?,
            "esc" | "escape" => self.back_or_exit(),
            _ => self.status = format!("Tuntematon näppäin: {key}"),
        }
        Ok(())
    }

    fn reload_config(&mut self, config_path: &Path) {
        let selected_id = self
            .buttons()
            .get(self.selected)
            .map(|button| button.id.clone());
        let previous_screen = self.screen.clone();

        match config::load(config_path) {
            Ok(config) => {
                let screen = if config.screen(&previous_screen).is_some() {
                    previous_screen
                } else {
                    config.home.clone()
                };

                self.config = config;
                self.screen = screen;
                self.selected = selected_id
                    .and_then(|id| self.buttons().iter().position(|button| button.id == id))
                    .unwrap_or(0);
                self.status = "Asetukset päivitetty.".to_owned();
            }
            Err(error) => {
                eprintln!(
                    "momarchy: could not reload {}: {error}; keeping previous config",
                    config_path.display()
                );
                self.status = "Asetusvirhe — vanhat asetukset säilytettiin.".to_owned();
            }
        }
    }

    fn write_snapshot(&self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "SCREEN {}", self.screen)?;
        writeln!(
            out,
            "MODE {}",
            if self.live_actions { "live" } else { "dry-run" }
        )?;
        writeln!(out, "SELECTED {}", self.buttons()[self.selected].id)?;
        writeln!(out, "ACTIONS")?;
        for button in self.buttons() {
            writeln!(
                out,
                "{}\t{}\t{}\t{}",
                button.id,
                button.action.kind(),
                button.label,
                button.hint
            )?;
        }
        writeln!(out, "STATUS {}", self.status)?;
        writeln!(out, "END")?;
        out.flush()
    }
}

fn spawn_terminal_reader(sender: Sender<RuntimeEvent>) -> io::Result<()> {
    std::thread::Builder::new()
        .name("momarchy-terminal-input".to_owned())
        .stack_size(256 * 1024)
        .spawn(move || {
            loop {
                match event::read() {
                    Ok(event) => {
                        if sender.send(RuntimeEvent::Terminal(event)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(RuntimeEvent::TerminalFailed(error.to_string()));
                        break;
                    }
                }
            }
        })?;
    Ok(())
}

fn run_automation(app: &mut App, config_path: &Path) -> io::Result<()> {
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    app.write_snapshot(&mut out)?;

    for line in stdin.lock().lines() {
        let line = line?;
        let command = line.trim();
        if command.is_empty() {
            continue;
        }

        if command == "quit" {
            break;
        } else if command == "activate" {
            app.activate()?;
        } else if command == "reload" {
            app.reload_config(config_path);
        } else if command == "snapshot" || command == "render" {
            // The semantic snapshot is the first automation surface. A full Ratatui
            // frame dump can be added later without changing the command stream.
        } else if let Some(id) = command.strip_prefix("select ") {
            if !app.select_id(id.trim()) {
                app.status = format!("Tuntematon valinta: {}", id.trim());
            }
        } else if let Some(key) = command.strip_prefix("key ") {
            app.automation_key(key.trim())?;
        } else {
            app.status = format!("Tuntematon automaatiokomento: {command}");
        }

        app.write_snapshot(&mut out)?;
        if app.exit {
            break;
        }
    }

    Ok(())
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
    let left = area.width.saturating_sub(width) / 2;

    Rect {
        x: area.x.saturating_add(left),
        y: area.y,
        width,
        height: area.height,
    }
}

fn grid_areas(area: Rect, count: usize, columns: u16, gap: u16) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }

    let columns = columns.max(1);
    let rows = count.div_ceil(usize::from(columns)) as u16;
    let natural_height = rows
        .saturating_mul(BUTTON_HEIGHT)
        .saturating_add(gap.saturating_mul(rows.saturating_sub(1)));
    let use_natural_height = area.height >= natural_height;
    let top = if use_natural_height {
        area.y
            .saturating_add(area.height.saturating_sub(natural_height) / 2)
    } else {
        area.y
    };
    let mut areas = Vec::with_capacity(count);

    for index in 0..count {
        let index = index as u16;
        let row = index / columns;
        let column = index % columns;
        let (x, width) = segment(area.x, area.width, columns, gap, column);
        let (y, height) = if use_natural_height {
            (
                top.saturating_add(row.saturating_mul(BUTTON_HEIGHT.saturating_add(gap))),
                BUTTON_HEIGHT,
            )
        } else {
            segment(area.y, area.height, rows, gap, row)
        };
        areas.push(Rect {
            x,
            y,
            width,
            height,
        });
    }

    areas
}

fn segment(origin: u16, length: u16, parts: u16, gap: u16, index: u16) -> (u16, u16) {
    let parts = parts.max(1);
    let total_gap = gap.saturating_mul(parts.saturating_sub(1));
    let usable = length.saturating_sub(total_gap);
    let base = usable / parts;
    let remainder = usable % parts;
    let extra_before = index.min(remainder);
    let extra_here = if index < remainder { 1 } else { 0 };
    let offset = index
        .saturating_mul(base)
        .saturating_add(extra_before)
        .saturating_add(index.saturating_mul(gap));

    (
        origin.saturating_add(offset),
        base.saturating_add(extra_here),
    )
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
    fn grid_uses_natural_button_height_when_space_allows() {
        let areas = grid_areas(Rect::new(0, 0, 21, 11), 4, 2, 1);
        assert_eq!(areas.len(), 4);
        assert_eq!(areas[0], Rect::new(0, 1, 10, 4));
        assert_eq!(areas[1], Rect::new(11, 1, 10, 4));
        assert_eq!(areas[2], Rect::new(0, 6, 10, 4));
        assert_eq!(areas[3], Rect::new(11, 6, 10, 4));
    }

    #[test]
    fn grid_compresses_only_when_natural_height_does_not_fit() {
        let areas = grid_areas(Rect::new(0, 0, 21, 7), 4, 2, 1);
        assert_eq!(areas[0], Rect::new(0, 0, 10, 3));
        assert_eq!(areas[1], Rect::new(11, 0, 10, 3));
        assert_eq!(areas[2], Rect::new(0, 4, 10, 3));
        assert_eq!(areas[3], Rect::new(11, 4, 10, 3));
    }

    #[test]
    fn menu_width_is_centered_and_capped() {
        assert_eq!(
            centered_max_width(Rect::new(10, 2, 100, 20), 64),
            Rect::new(28, 2, 64, 20)
        );
        assert_eq!(
            centered_max_width(Rect::new(10, 2, 50, 20), 64),
            Rect::new(10, 2, 50, 20)
        );
    }
}
