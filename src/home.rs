use std::{
    io::{self, BufRead, Write, stdout},
    process::Command,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub automation: bool,
    pub live_actions: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Games,
    Help,
}

#[derive(Clone, Copy)]
enum Action {
    Navigate(Screen),
    Message(&'static str),
    Host {
        kind: &'static str,
        program: &'static str,
        args: &'static [&'static str],
        live_message: &'static str,
    },
}

#[derive(Clone, Copy)]
struct Button {
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    action: Action,
}

const HOME_BUTTONS: [Button; 8] = [
    Button {
        id: "internet",
        label: "INTERNET",
        hint: "Avaa selain",
        action: Action::Host {
            kind: "browser",
            program: "xdg-open",
            args: &["https://www.google.fi/"],
            live_message: "Avataan internet.",
        },
    },
    Button {
        id: "email",
        label: "SÄHKÖPOSTI",
        hint: "Lue ja lähetä viestejä",
        action: Action::Message("Sähköposti otetaan käyttöön seuraavaksi."),
    },
    Button {
        id: "photos",
        label: "KUVAT",
        hint: "Katso kuvia",
        action: Action::Message("Kuvat otetaan käyttöön seuraavaksi."),
    },
    Button {
        id: "youtube",
        label: "YOUTUBE",
        hint: "Katso videoita",
        action: Action::Host {
            kind: "browser",
            program: "xdg-open",
            args: &["https://www.youtube.com/"],
            live_message: "Avataan YouTube.",
        },
    },
    Button {
        id: "ask",
        label: "KYSY MITÄ VAIN",
        hint: "Kirjoita tai puhu kysymys",
        action: Action::Message("Kysy mitä vain tulee seuraavaksi."),
    },
    Button {
        id: "tv",
        label: "KATSO TELEVISIOSTA",
        hint: "Chromecast",
        action: Action::Message("Chromecast-tuki tulee seuraavaksi."),
    },
    Button {
        id: "games",
        label: "PELIT",
        hint: "Palikat, Mato...",
        action: Action::Navigate(Screen::Games),
    },
    Button {
        id: "help",
        label: "APUA",
        hint: "Jos jokin ei toimi",
        action: Action::Navigate(Screen::Help),
    },
];

const GAME_BUTTONS: [Button; 3] = [
    Button {
        id: "palikat",
        label: "PALIKAT",
        hint: "Putoavia palikoita",
        action: Action::Message("Palikat tulee pian :)"),
    },
    Button {
        id: "mato",
        label: "MATO",
        hint: "Syö ja kasva",
        action: Action::Message("Mato tulee pian :)"),
    },
    Button {
        id: "back",
        label: "TAKAISIN",
        hint: "Palaa alkuun",
        action: Action::Navigate(Screen::Home),
    },
];

const HELP_BUTTONS: [Button; 1] = [Button {
    id: "back",
    label: "TAKAISIN",
    hint: "Palaa alkuun",
    action: Action::Navigate(Screen::Home),
}];

pub fn run(options: Options) -> io::Result<()> {
    if options.automation {
        return run_automation(options);
    }

    let _mouse = MouseCaptureGuard::enable()?;
    ratatui::run(|terminal| App::new(options.live_actions).run(terminal))
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

struct App {
    screen: Screen,
    selected: usize,
    button_areas: Vec<Rect>,
    status: String,
    exit: bool,
    live_actions: bool,
}

impl App {
    fn new(live_actions: bool) -> Self {
        Self {
            screen: Screen::Home,
            selected: 0,
            button_areas: Vec::new(),
            status: "Valitse hiirellä tai nuolinäppäimillä.".to_owned(),
            exit: false,
            live_actions,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event(event::read()?)?;
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

        let (title, subtitle) = match self.screen {
            Screen::Home => ("MOMARCHY", "Mitä haluat tehdä?"),
            Screen::Games => ("PELIT", "Valitse peli"),
            Screen::Help => ("APUA", "Jos jokin ei toimi"),
        };

        let header = Paragraph::new(format!("{title}\n{subtitle}"))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(header, areas[0]);

        if self.screen == Screen::Help {
            let help_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(6), Constraint::Length(6)])
                .split(areas[1]);

            let help = Paragraph::new(
                "Momarchy tarkistaa myöhemmin tästä internet-yhteyden ja muut tärkeät asiat.\n\nJos jokin ei toimi, pyydä apua.",
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
            frame.render_widget(help, help_areas[0]);
            self.render_buttons(frame, help_areas[1]);
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
        let buttons = self.buttons();
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
                let button = buttons[index];
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

    fn buttons(&self) -> &'static [Button] {
        match self.screen {
            Screen::Home => &HOME_BUTTONS,
            Screen::Games => &GAME_BUTTONS,
            Screen::Help => &HELP_BUTTONS,
        }
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
        if self.screen == Screen::Home {
            self.exit = true;
        } else {
            self.go_to(Screen::Home);
        }
    }

    fn go_to(&mut self, screen: Screen) {
        self.screen = screen;
        self.selected = 0;
        self.status = match screen {
            Screen::Home => "Mitä haluat tehdä?".to_owned(),
            Screen::Games => "Valitse peli.".to_owned(),
            Screen::Help => "Apua ja tarkistukset tulevat tähän.".to_owned(),
        };
    }

    fn activate(&mut self) -> io::Result<()> {
        let button = self.buttons()[self.selected];
        match button.action {
            Action::Navigate(screen) => self.go_to(screen),
            Action::Message(message) => self.status = message.to_owned(),
            Action::Host {
                kind,
                program,
                args,
                live_message,
            } => {
                if self.live_actions {
                    Command::new(program).args(args).spawn()?;
                    self.status = live_message.to_owned();
                } else {
                    self.status = format!(
                        "KEHITYSTILA — {kind}: {} {}",
                        program,
                        args.join(" ")
                    );
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

    fn write_snapshot(&self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "SCREEN {}", screen_id(self.screen))?;
        writeln!(out, "MODE {}", if self.live_actions { "live" } else { "dry-run" })?;
        writeln!(out, "SELECTED {}", self.buttons()[self.selected].id)?;
        writeln!(out, "ACTIONS")?;
        for button in self.buttons() {
            writeln!(
                out,
                "{}\t{}\t{}\t{}",
                button.id,
                action_kind(button.action),
                button.label,
                button.hint
            )?;
        }
        writeln!(out, "STATUS {}", self.status)?;
        writeln!(out, "END")?;
        out.flush()
    }
}

fn run_automation(options: Options) -> io::Result<()> {
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    let mut app = App::new(options.live_actions);
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

fn screen_id(screen: Screen) -> &'static str {
    match screen {
        Screen::Home => "home",
        Screen::Games => "games",
        Screen::Help => "help",
    }
}

fn action_kind(action: Action) -> &'static str {
    match action {
        Action::Navigate(_) => "internal",
        Action::Message(_) => "internal",
        Action::Host { kind, .. } => kind,
    }
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}
