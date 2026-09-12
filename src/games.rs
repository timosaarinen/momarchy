use std::{
    collections::VecDeque,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const PALIKAT_WIDTH: usize = 10;
const PALIKAT_HEIGHT: usize = 20;
const MATO_WIDTH: i16 = 24;
const MATO_HEIGHT: i16 = 16;
const SIDEBAR_WIDTH: u16 = 16;
const GAME_GAP: u16 = 2;

#[derive(Clone, Copy)]
pub struct Styles {
    pub base: Style,
    pub muted: Style,
    pub accent: Style,
    pub panel: Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    None,
    Back,
}

#[derive(Default)]
pub struct State {
    game: Option<Game>,
    help_visible: bool,
}

impl State {
    pub fn start(&mut self, id: &str) -> Result<(), String> {
        let seed = random_seed();
        self.game = Some(match id {
            "palikat" => Game::Palikat(Palikat::new(seed)),
            "mato" => Game::Mato(Mato::new(seed)),
            _ => return Err(format!("unknown built-in game: {id}")),
        });
        self.help_visible = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.game = None;
        self.help_visible = false;
    }

    pub fn is_active(&self) -> bool {
        self.game.is_some()
    }

    pub fn active_name(&self) -> Option<&'static str> {
        match self.game.as_ref() {
            Some(Game::Palikat(_)) => Some("palikat"),
            Some(Game::Mato(_)) => Some("mato"),
            None => None,
        }
    }

    pub fn tick_delay(&self) -> Option<Duration> {
        if self.help_visible {
            return None;
        }
        match self.game.as_ref() {
            Some(Game::Palikat(game)) => game.tick_delay(),
            Some(Game::Mato(game)) => game.tick_delay(),
            None => None,
        }
    }

    pub fn tick(&mut self) {
        if self.help_visible {
            return;
        }
        match self.game.as_mut() {
            Some(Game::Palikat(game)) => game.tick(),
            Some(Game::Mato(game)) => game.tick(),
            None => {}
        }
    }

    pub fn handle_event(&mut self, event: Event) -> Intent {
        let Event::Key(key) = event else {
            return Intent::None;
        };
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Intent::None;
        }
        self.handle_key(key.code)
    }

    pub fn handle_named_key(&mut self, key: &str) -> Intent {
        let code = match key {
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "enter" => KeyCode::Enter,
            "space" => KeyCode::Char(' '),
            "h" | "help" => KeyCode::Char('h'),
            "p" => KeyCode::Char('p'),
            "r" => KeyCode::Char('r'),
            "esc" | "escape" => KeyCode::Esc,
            _ => return Intent::None,
        };
        self.handle_key(code)
    }

    fn handle_key(&mut self, code: KeyCode) -> Intent {
        if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
            self.help_visible = false;
            return Intent::Back;
        }

        if matches!(code, KeyCode::Char('h') | KeyCode::Char('?')) && self.game.is_some() {
            self.help_visible = !self.help_visible;
            return Intent::None;
        }

        if self.help_visible {
            return Intent::None;
        }

        match self.game.as_mut() {
            Some(Game::Palikat(game)) => game.handle_key(code),
            Some(Game::Mato(game)) => game.handle_key(code),
            None => {}
        }
        Intent::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        let Some(game) = self.game.as_ref() else {
            return;
        };

        if self.help_visible {
            render_help(frame, area, styles, game);
            return;
        }

        match game {
            Game::Palikat(game) => game.render(frame, area, styles),
            Game::Mato(game) => game.render(frame, area, styles),
        }
    }
}

enum Game {
    Palikat(Palikat),
    Mato(Mato),
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: usize,
    rotation: u8,
    x: i8,
    y: i8,
}

struct Palikat {
    board: [[bool; PALIKAT_WIDTH]; PALIKAT_HEIGHT],
    current: Piece,
    next: usize,
    rng: TinyRng,
    score: u32,
    lines: u32,
    paused: bool,
    game_over: bool,
}

impl Palikat {
    fn new(seed: u64) -> Self {
        let mut rng = TinyRng::new(seed);
        let current = Piece {
            kind: rng.index(7),
            rotation: 0,
            x: 3,
            y: 0,
        };
        let next = rng.index(7);
        Self {
            board: [[false; PALIKAT_WIDTH]; PALIKAT_HEIGHT],
            current,
            next,
            rng,
            score: 0,
            lines: 0,
            paused: false,
            game_over: false,
        }
    }

    fn restart(&mut self) {
        let seed = self.rng.next_u64();
        *self = Self::new(seed);
    }

    fn tick_delay(&self) -> Option<Duration> {
        if self.paused || self.game_over {
            return None;
        }
        let level = self.lines / 10;
        let millis = 650u64.saturating_sub(u64::from(level) * 45).max(150);
        Some(Duration::from_millis(millis))
    }

    fn tick(&mut self) {
        if self.paused || self.game_over {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock_piece();
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        if matches!(code, KeyCode::Char('r')) {
            self.restart();
            return;
        }
        if matches!(code, KeyCode::Char('p')) && !self.game_over {
            self.paused = !self.paused;
            return;
        }
        if self.game_over {
            if matches!(code, KeyCode::Enter) {
                self.restart();
            }
            return;
        }
        if self.paused {
            return;
        }

        match code {
            KeyCode::Left => {
                self.try_move(-1, 0);
            }
            KeyCode::Right => {
                self.try_move(1, 0);
            }
            KeyCode::Down => {
                if self.try_move(0, 1) {
                    self.score = self.score.saturating_add(1);
                } else {
                    self.lock_piece();
                }
            }
            KeyCode::Up | KeyCode::Enter => self.rotate(),
            KeyCode::Char(' ') => self.hard_drop(),
            _ => {}
        }
    }

    fn try_move(&mut self, dx: i8, dy: i8) -> bool {
        let mut candidate = self.current;
        candidate.x = candidate.x.saturating_add(dx);
        candidate.y = candidate.y.saturating_add(dy);
        if self.fits(candidate) {
            self.current = candidate;
            true
        } else {
            false
        }
    }

    fn rotate(&mut self) {
        let mut candidate = self.current;
        candidate.rotation = (candidate.rotation + 1) % 4;
        for kick in [0, -1, 1, -2, 2] {
            candidate.x = self.current.x.saturating_add(kick);
            if self.fits(candidate) {
                self.current = candidate;
                return;
            }
        }
    }

    fn hard_drop(&mut self) {
        let mut distance = 0u32;
        while self.try_move(0, 1) {
            distance += 1;
        }
        self.score = self.score.saturating_add(distance.saturating_mul(2));
        self.lock_piece();
    }

    fn fits(&self, piece: Piece) -> bool {
        shape_cells(piece.kind, piece.rotation).into_iter().all(|(dx, dy)| {
            let x = i16::from(piece.x) + i16::from(dx);
            let y = i16::from(piece.y) + i16::from(dy);
            if x < 0 || x >= PALIKAT_WIDTH as i16 || y >= PALIKAT_HEIGHT as i16 {
                return false;
            }
            y < 0 || !self.board[y as usize][x as usize]
        })
    }

    fn lock_piece(&mut self) {
        for (dx, dy) in shape_cells(self.current.kind, self.current.rotation) {
            let x = i16::from(self.current.x) + i16::from(dx);
            let y = i16::from(self.current.y) + i16::from(dy);
            if y < 0 {
                self.game_over = true;
                return;
            }
            if (0..PALIKAT_WIDTH as i16).contains(&x) && y < PALIKAT_HEIGHT as i16 {
                self.board[y as usize][x as usize] = true;
            }
        }

        let cleared = self.clear_full_rows();
        self.lines = self.lines.saturating_add(cleared);
        self.score = self.score.saturating_add(match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        });

        self.current = Piece {
            kind: self.next,
            rotation: 0,
            x: 3,
            y: 0,
        };
        self.next = self.rng.index(7);
        if !self.fits(self.current) {
            self.game_over = true;
        }
    }

    fn clear_full_rows(&mut self) -> u32 {
        let mut cleared = 0;
        let mut y = PALIKAT_HEIGHT as isize - 1;
        while y >= 0 {
            let row = y as usize;
            if self.board[row].iter().all(|cell| *cell) {
                for target in (1..=row).rev() {
                    self.board[target] = self.board[target - 1];
                }
                self.board[0] = [false; PALIKAT_WIDTH];
                cleared += 1;
            } else {
                y -= 1;
            }
        }
        cleared
    }

    fn cell_is_falling(&self, x: usize, y: usize) -> bool {
        shape_cells(self.current.kind, self.current.rotation)
            .into_iter()
            .any(|(dx, dy)| {
                i16::from(self.current.x) + i16::from(dx) == x as i16
                    && i16::from(self.current.y) + i16::from(dy) == y as i16
            })
    }

    fn render(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        let outer = inset(area, 1);
        let board_height = PALIKAT_HEIGHT as u16 + 2;
        if outer.height < board_height {
            render_too_small(frame, outer, styles, "PALIKAT", "Ikkuna on liian matala pelille.");
            return;
        }

        let cell_width = if outer.width >= PALIKAT_WIDTH as u16 * 2 + 2 { 2 } else { 1 };
        let board_width = PALIKAT_WIDTH as u16 * cell_width + 2;
        if outer.width < board_width {
            render_too_small(frame, outer, styles, "PALIKAT", "Ikkuna on liian kapea pelille.");
            return;
        }

        let with_sidebar = outer.width >= board_width + GAME_GAP + SIDEBAR_WIDTH;
        let total_width = if with_sidebar {
            board_width + GAME_GAP + SIDEBAR_WIDTH
        } else {
            board_width
        };
        let game_area = centered_size(outer, total_width, board_height);
        let board_area = Rect::new(game_area.x, game_area.y, board_width, board_height);

        let mut lines = Vec::with_capacity(PALIKAT_HEIGHT);
        for y in 0..PALIKAT_HEIGHT {
            let mut spans = Vec::with_capacity(PALIKAT_WIDTH);
            for x in 0..PALIKAT_WIDTH {
                let glyph = if cell_width == 2 { "██" } else { "█" };
                let empty = if cell_width == 2 { "  " } else { " " };
                if self.cell_is_falling(x, y) && !self.game_over {
                    spans.push(Span::styled(glyph, styles.accent.add_modifier(Modifier::BOLD)));
                } else if self.board[y][x] {
                    spans.push(Span::styled(glyph, styles.base.add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw(empty));
                }
            }
            lines.push(Line::from(spans));
        }
        let board = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).style(styles.base))
            .style(styles.base);
        frame.render_widget(board, board_area);

        if with_sidebar {
            let sidebar = Rect::new(
                board_area.x + board_area.width + GAME_GAP,
                board_area.y,
                SIDEBAR_WIDTH,
                board_area.height,
            );
            self.render_sidebar(frame, sidebar, styles);
        }
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        render_text_row(frame, Rect::new(area.x, area.y, area.width, 1), "PALIKAT", styles.accent.add_modifier(Modifier::BOLD));
        render_stat(frame, area, 2, "PISTEET", self.score.to_string(), styles);
        render_stat(frame, area, 6, "TASO", (self.lines / 10 + 1).to_string(), styles);
        render_stat(frame, area, 10, "RIVIT", self.lines.to_string(), styles);

        let next_label = Rect::new(area.x, area.y + 14, area.width, 1);
        render_text_row(frame, next_label, "SEURAAVA", styles.panel.add_modifier(Modifier::BOLD));
        let preview = Rect::new(area.x, area.y + 15, area.width, 4.min(area.height.saturating_sub(15)));
        render_piece_preview(frame, preview, self.next, styles.base.add_modifier(Modifier::BOLD));

        let state = if self.game_over {
            Some("PELI PÄÄTTYI\nENTER = UUSI")
        } else if self.paused {
            Some("TAUKO")
        } else {
            None
        };
        if let Some(state) = state {
            let state_area = Rect::new(area.x, area.y + 19, area.width, area.height.saturating_sub(19));
            let widget = Paragraph::new(state)
                .alignment(Alignment::Center)
                .style(styles.accent.add_modifier(Modifier::BOLD));
            frame.render_widget(widget, state_area);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Up, Self::Down)
                | (Self::Down, Self::Up)
                | (Self::Left, Self::Right)
                | (Self::Right, Self::Left)
        )
    }
}

struct Mato {
    snake: VecDeque<(i16, i16)>,
    direction: Direction,
    queued_direction: Direction,
    food: (i16, i16),
    rng: TinyRng,
    score: u32,
    paused: bool,
    game_over: bool,
}

impl Mato {
    fn new(seed: u64) -> Self {
        let mut game = Self {
            snake: VecDeque::from([(12, 8), (11, 8), (10, 8), (9, 8)]),
            direction: Direction::Right,
            queued_direction: Direction::Right,
            food: (18, 8),
            rng: TinyRng::new(seed),
            score: 0,
            paused: false,
            game_over: false,
        };
        game.place_food();
        game
    }

    fn restart(&mut self) {
        let seed = self.rng.next_u64();
        *self = Self::new(seed);
    }

    fn tick_delay(&self) -> Option<Duration> {
        if self.paused || self.game_over {
            return None;
        }
        let speedup = u64::from(self.score.min(20)) * 4;
        Some(Duration::from_millis(170u64.saturating_sub(speedup).max(90)))
    }

    fn handle_key(&mut self, code: KeyCode) {
        if matches!(code, KeyCode::Char('r')) {
            self.restart();
            return;
        }
        if matches!(code, KeyCode::Char('p')) && !self.game_over {
            self.paused = !self.paused;
            return;
        }
        if self.game_over {
            if matches!(code, KeyCode::Enter) {
                self.restart();
            }
            return;
        }
        if self.paused {
            return;
        }

        let requested = match code {
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        };
        if let Some(direction) = requested
            && !self.direction.opposite(direction)
        {
            self.queued_direction = direction;
        }
    }

    fn tick(&mut self) {
        if self.paused || self.game_over {
            return;
        }
        self.direction = self.queued_direction;
        let Some(&(head_x, head_y)) = self.snake.front() else {
            self.game_over = true;
            return;
        };
        let (dx, dy) = match self.direction {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        };
        let next = (head_x + dx, head_y + dy);
        if next.0 < 0 || next.0 >= MATO_WIDTH || next.1 < 0 || next.1 >= MATO_HEIGHT {
            self.game_over = true;
            return;
        }

        let grows = next == self.food;
        let tail = self.snake.back().copied();
        let hits_self = self
            .snake
            .iter()
            .any(|segment| *segment == next && (grows || Some(*segment) != tail));
        if hits_self {
            self.game_over = true;
            return;
        }

        self.snake.push_front(next);
        if grows {
            self.score = self.score.saturating_add(1);
            self.place_food();
        } else {
            self.snake.pop_back();
        }
    }

    fn place_food(&mut self) {
        for _ in 0..256 {
            let candidate = (
                self.rng.index(MATO_WIDTH as usize) as i16,
                self.rng.index(MATO_HEIGHT as usize) as i16,
            );
            if !self.snake.contains(&candidate) {
                self.food = candidate;
                return;
            }
        }
        for y in 0..MATO_HEIGHT {
            for x in 0..MATO_WIDTH {
                if !self.snake.contains(&(x, y)) {
                    self.food = (x, y);
                    return;
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        let outer = inset(area, 1);
        let board_height = MATO_HEIGHT as u16 + 2;
        if outer.height < board_height {
            render_too_small(frame, outer, styles, "MATO", "Ikkuna on liian matala pelille.");
            return;
        }

        let wide_board = MATO_WIDTH as u16 * 2 + 2;
        let narrow_board = MATO_WIDTH as u16 + 2;
        let cell_width = if outer.width >= wide_board { 2 } else { 1 };
        let board_width = if cell_width == 2 { wide_board } else { narrow_board };
        if outer.width < board_width {
            render_too_small(frame, outer, styles, "MATO", "Ikkuna on liian kapea pelille.");
            return;
        }

        let with_sidebar = outer.width >= board_width + GAME_GAP + SIDEBAR_WIDTH;
        let total_width = if with_sidebar {
            board_width + GAME_GAP + SIDEBAR_WIDTH
        } else {
            board_width
        };
        let game_area = centered_size(outer, total_width, board_height);
        let board_area = Rect::new(game_area.x, game_area.y, board_width, board_height);
        let head = self.snake.front().copied();
        let mut lines = Vec::with_capacity(MATO_HEIGHT as usize);
        for y in 0..MATO_HEIGHT {
            let mut spans = Vec::with_capacity(MATO_WIDTH as usize);
            for x in 0..MATO_WIDTH {
                let point = (x, y);
                let block = if cell_width == 2 { "██" } else { "█" };
                let food = if cell_width == 2 { "● " } else { "●" };
                let empty = if cell_width == 2 { "  " } else { " " };
                if Some(point) == head {
                    spans.push(Span::styled(block, styles.accent.add_modifier(Modifier::BOLD)));
                } else if self.snake.contains(&point) {
                    spans.push(Span::styled(block, styles.base.add_modifier(Modifier::BOLD)));
                } else if point == self.food {
                    spans.push(Span::styled(food, styles.accent.add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw(empty));
                }
            }
            lines.push(Line::from(spans));
        }
        let board = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).style(styles.base))
            .style(styles.base);
        frame.render_widget(board, board_area);

        if with_sidebar {
            let sidebar = Rect::new(
                board_area.x + board_area.width + GAME_GAP,
                board_area.y,
                SIDEBAR_WIDTH,
                board_area.height,
            );
            self.render_sidebar(frame, sidebar, styles);
        }
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        render_text_row(frame, Rect::new(area.x, area.y, area.width, 1), "MATO", styles.accent.add_modifier(Modifier::BOLD));
        render_stat(frame, area, 3, "PISTEET", self.score.to_string(), styles);
        render_stat(frame, area, 8, "PITUUS", self.snake.len().to_string(), styles);

        let state = if self.game_over {
            Some("PELI PÄÄTTYI\nENTER = UUSI")
        } else if self.paused {
            Some("TAUKO")
        } else {
            None
        };
        if let Some(state) = state {
            let state_area = Rect::new(area.x, area.y + 14, area.width, area.height.saturating_sub(14));
            let widget = Paragraph::new(state)
                .alignment(Alignment::Center)
                .style(styles.accent.add_modifier(Modifier::BOLD));
            frame.render_widget(widget, state_area);
        }
    }
}

#[derive(Clone, Copy)]
struct TinyRng(u64);

impl TinyRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next_u64() % upper as u64) as usize
    }
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x4d4f_4d41_5243_4859)
        ^ u64::from(std::process::id())
}

fn render_help(frame: &mut Frame, area: Rect, styles: Styles, game: &Game) {
    let outer = inset(area, 2);
    let width = outer.width.min(54);
    let height = match game {
        Game::Palikat(_) => 10,
        Game::Mato(_) => 7,
    }
    .min(outer.height);
    let panel = centered_size(outer, width, height);
    let title_area = Rect::new(panel.x, panel.y, panel.width, 1.min(panel.height));
    let title = match game {
        Game::Palikat(_) => "PALIKAT — OHJE",
        Game::Mato(_) => "MATO — OHJE",
    };
    render_text_row(frame, title_area, title, styles.panel.add_modifier(Modifier::BOLD));

    if panel.height <= 1 {
        return;
    }
    let body_area = Rect::new(panel.x, panel.y + 2.min(panel.height), panel.width, panel.height.saturating_sub(2));
    let body = match game {
        Game::Palikat(_) => {
            "← / →  liikuta\n↑ tai Enter  käännä\n↓  alas\nVälilyönti  pudota\nP  tauko\nR  uusi peli\nH  sulje ohje"
        }
        Game::Mato(_) => "Nuolinäppäimet ohjaavat\nP  tauko\nR  uusi peli\nH  sulje ohje",
    };
    let widget = Paragraph::new(body)
        .alignment(Alignment::Center)
        .style(styles.base)
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, body_area);
}

fn render_too_small(frame: &mut Frame, area: Rect, styles: Styles, title: &str, message: &str) {
    let panel = centered_size(area, area.width.min(44), area.height.min(5));
    let widget = Paragraph::new(format!("{title}\n\n{message}"))
        .alignment(Alignment::Center)
        .style(styles.base.add_modifier(Modifier::BOLD))
        .wrap(Wrap { trim: true });
    frame.render_widget(widget, panel);
}

fn render_stat(frame: &mut Frame, area: Rect, offset_y: u16, label: &str, value: String, styles: Styles) {
    if offset_y >= area.height {
        return;
    }
    let label_area = Rect::new(area.x, area.y + offset_y, area.width, 1);
    render_text_row(frame, label_area, label, styles.panel.add_modifier(Modifier::BOLD));
    if offset_y + 1 < area.height {
        let value_area = Rect::new(area.x, area.y + offset_y + 1, area.width, 1);
        render_text_row(frame, value_area, &value, styles.base.add_modifier(Modifier::BOLD));
    }
}

fn render_text_row(frame: &mut Frame, area: Rect, text: &str, style: Style) {
    let widget = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(style);
    frame.render_widget(widget, area);
}

fn render_piece_preview(frame: &mut Frame, area: Rect, kind: usize, style: Style) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let cells = shape_cells(kind, 0);
    let mut lines = Vec::with_capacity(4);
    for y in 0..4i8 {
        let mut spans = Vec::with_capacity(4);
        for x in 0..4i8 {
            if cells.contains(&(x, y)) {
                spans.push(Span::styled("██", style));
            } else {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }
    let widget = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(style);
    frame.render_widget(widget, area);
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

fn centered_size(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x.saturating_add(area.width.saturating_sub(width) / 2),
        y: area.y.saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    }
}

fn shape_cells(kind: usize, rotation: u8) -> [(i8, i8); 4] {
    match (kind, rotation % 4) {
        (0, 0 | 2) => [(0, 1), (1, 1), (2, 1), (3, 1)],
        (0, 1 | 3) => [(2, 0), (2, 1), (2, 2), (2, 3)],
        (1, _) => [(1, 0), (2, 0), (1, 1), (2, 1)],
        (2, 0) => [(1, 0), (0, 1), (1, 1), (2, 1)],
        (2, 1) => [(1, 0), (1, 1), (2, 1), (1, 2)],
        (2, 2) => [(0, 1), (1, 1), (2, 1), (1, 2)],
        (2, 3) => [(1, 0), (0, 1), (1, 1), (1, 2)],
        (3, 0) => [(1, 0), (2, 0), (0, 1), (1, 1)],
        (3, 1) => [(1, 0), (1, 1), (2, 1), (2, 2)],
        (3, 2) => [(1, 1), (2, 1), (0, 2), (1, 2)],
        (3, 3) => [(0, 0), (0, 1), (1, 1), (1, 2)],
        (4, 0) => [(0, 0), (1, 0), (1, 1), (2, 1)],
        (4, 1) => [(2, 0), (1, 1), (2, 1), (1, 2)],
        (4, 2) => [(0, 1), (1, 1), (1, 2), (2, 2)],
        (4, 3) => [(1, 0), (0, 1), (1, 1), (0, 2)],
        (5, 0) => [(0, 0), (0, 1), (1, 1), (2, 1)],
        (5, 1) => [(1, 0), (2, 0), (1, 1), (1, 2)],
        (5, 2) => [(0, 1), (1, 1), (2, 1), (2, 2)],
        (5, 3) => [(1, 0), (1, 1), (0, 2), (1, 2)],
        (6, 0) => [(2, 0), (0, 1), (1, 1), (2, 1)],
        (6, 1) => [(1, 0), (1, 1), (1, 2), (2, 2)],
        (6, 2) => [(0, 1), (1, 1), (2, 1), (0, 2)],
        (6, 3) => [(0, 0), (1, 0), (1, 1), (1, 2)],
        _ => unreachable!("tetromino kind must be 0..7"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palikat_clears_full_rows() {
        let mut game = Palikat::new(1);
        game.board[PALIKAT_HEIGHT - 1] = [true; PALIKAT_WIDTH];
        assert_eq!(game.clear_full_rows(), 1);
        assert!(game.board[PALIKAT_HEIGHT - 1].iter().all(|cell| !cell));
    }

    #[test]
    fn palikat_wall_collision_blocks_move() {
        let mut game = Palikat::new(1);
        game.current = Piece {
            kind: 1,
            rotation: 0,
            x: -1,
            y: 0,
        };
        assert!(!game.try_move(-1, 0));
    }

    #[test]
    fn help_pauses_game_ticks() {
        let mut state = State::default();
        state.start("palikat").unwrap();
        assert!(state.tick_delay().is_some());
        state.handle_named_key("h");
        assert!(state.tick_delay().is_none());
        state.handle_named_key("h");
        assert!(state.tick_delay().is_some());
    }

    #[test]
    fn mato_rejects_reverse_direction() {
        let mut game = Mato::new(1);
        game.handle_key(KeyCode::Left);
        assert_eq!(game.queued_direction, Direction::Right);
        game.handle_key(KeyCode::Up);
        assert_eq!(game.queued_direction, Direction::Up);
    }

    #[test]
    fn mato_grows_when_food_is_eaten() {
        let mut game = Mato::new(1);
        let before = game.snake.len();
        let head = *game.snake.front().unwrap();
        game.food = (head.0 + 1, head.1);
        game.tick();
        assert_eq!(game.snake.len(), before + 1);
        assert_eq!(game.score, 1);
    }
}
