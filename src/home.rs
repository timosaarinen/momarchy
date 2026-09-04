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
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    config::{self, Action, Button, Config},
    watch::{self, WatchEvent},
};

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub automation: bool,
    pub live_actions: bool,
}

pub fn run(options: Options) -> io::Result<()> {
    let initial = config::initialize()?;
    let mut app = App::new(initial.config, options.live_actions, initial.warning);

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
    WatchFailed(String),
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
    fn new(config: Config, live_actions: bool, warning: Option<String>) -> Self {
        let screen = config.home.clone();
        Self {
            config,
            screen,
            selected: 0,
            button_areas: Vec::new(),
            status: warning.unwrap_or_else(|| "Valitse hiirellä tai nuolinäppäimillä.".to_owned()),
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
                WatchEvent::Failed(error) => RuntimeEvent::WatchFailed(error),
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
                Ok(RuntimeEvent::WatchFailed(error)) => {
                    self.status = format!("Asetusten automaattinen päivitys ei toimi: {error}");
                }
                Err(_) => return Err(io::Error::other("Momarchy event sources stopped")),
            }
        }

        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(16),
                Constraint::Length(4),
            ])
            .split(frame.area());

        let (title, subtitle, body) = {
            let screen = self.current_screen();
            (
                screen.title.clone(),
                screen.subtitle.clone(),
                screen.body.clone(),
            )
        };

        let header = Paragraph::new(format!("{title}\n{subtitle}"))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(header, areas[0]);

        if let Some(body) = body {
            let content_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(6)])
                .split(areas[1]);

            let body = Paragraph::new(body)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(body, content_areas[0]);
            self.render_buttons(frame, content_areas[1]);
        } else {
            self.render_buttons(frame, areas[1]);
        }

        let mode = if self.live_actions {
            ""
        } else {
            "KEHITYSTILA — ulkoisia ohjelmia ei käynnistetä\n"
        };
        let footer = Paragraph::new(format!("{mode}{}", self.status))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, areas[2]);
    }

    fn render_buttons(&mut self, frame: &mut Frame, area: Rect) {
        let buttons = self.buttons().to_vec();
        self.button_areas.clear();
        self.button_areas.resize(buttons.len(), Rect::default());

        let rows_count = buttons.len().div_ceil(2);
        let row_constraints = vec![Constraint::Ratio(1, rows_count as u32); rows_count];
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(area);

        for (row_index, row) in rows.iter().enumerate() {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(*row);

            for (column_index, button_area) in columns.iter().enumerate() {
                let index = row_index * 2 + column_index;
                if index >= buttons.len() {
                    continue;
                }

                self.button_areas[index] = *button_area;
                let button = &buttons[index];
                let selected = index == self.selected;
                let style = if selected {
                    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default()
                };

                let block = Block::default().borders(Borders::ALL).style(style);
                let text = format!("\n{}\n{}", button.label, button.hint);
                let widget = Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .block(block)
                    .style(style)
                    .wrap(Wrap { trim: true });
                frame.render_widget(widget, *button_area);
            }
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
        if self.selected % 2 == 1 {
            self.selected -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.selected % 2 == 0 && self.selected + 1 < self.buttons().len() {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        if self.selected >= 2 {
            self.selected -= 2;
        }
    }

    fn move_down(&mut self) {
        if self.selected + 2 < self.buttons().len() {
            self.selected += 2;
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
                    Command::new("xdg-open").arg(&target).spawn()?;
                    self.status = live_message;
                } else {
                    self.status = format!("KEHITYSTILA — browser: xdg-open {target}");
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
                    self.status = format!("KEHITYSTILA — {kind}: {program} {}", args.join(" "));
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
                self.status = format!("Asetusvirhe — vanhat asetukset säilytettiin: {error}");
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

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}
