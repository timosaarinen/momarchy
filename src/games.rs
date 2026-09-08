use std::{collections::VecDeque, time::{Duration, SystemTime, UNIX_EPOCH}};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

const PALIKAT_WIDTH: usize = 10;
const PALIKAT_HEIGHT: usize = 20;
const MATO_WIDTH: i16 = 24;
const MATO_HEIGHT: i16 = 16;

#[derive(Clone, Copy)]
pub struct Styles {
    pub base: Style,
    pub muted: Style,
    pub accent: Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    None,
    Back,
}

#[derive(Default)]
pub struct State {
    game: Option<Game>,
}

impl State {
    pub fn start(&mut self, id: &str) -> Result<(), String> {
        let seed = random_seed();
        self.game = Some(match id {
            "palikat" => Game::Palikat(Palikat::new(seed)),
            "mato" => Game::Mato(Mato::new(seed)),
            _ => return Err(format!("unknown built-in game: {id}")),
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        self.game = None;
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
        match self.game.as_ref() {
            Some(Game::Palikat(game)) => game.tick_delay(),
            Some(Game::Mato(game)) => game.tick_delay(),
            None => None,
        }
    }

    pub fn tick(&mut self) {
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
            "p" => KeyCode::Char('p'),
            "r" => KeyCode::Char('r'),
            "esc" | "escape" => KeyCode::Esc,
            _ => return Intent::None,
        };
        self.handle_key(code)
    }

    fn handle_key(&mut self, code: KeyCode) -> Intent {
        if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
            return Intent::Back;
        }

        match self.game.as_mut() {
            Some(Game::Palikat(game)) => game.handle_key(code),
            Some(Game::Mato(game)) => game.handle_key(code),
            None => {}
        }
        Intent::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, styles: Styles) {
        match self.game.as_ref() {
            Some(Game::Palikat(game)) => game.render(frame, area, styles),
            Some(Game::Mato(game)) => game.render(frame, area, styles),
            None => {}
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
        let rows = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .split(outer);

        let title = Paragraph::new(vec![
            Line::from(Span::styled("PALIKAT", styles.base.add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(
                format!("Pisteet {}   Rivit {}   Taso {}", self.score, self.lines, self.lines / 10 + 1),
                styles.muted,
            )),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(title, rows[0]);

        let board_area = centered_size(rows[1], 22, 22);
        let mut lines = Vec::with_capacity(PALIKAT_HEIGHT);
        for y in 0..PALIKAT_HEIGHT {
            let mut spans = Vec::with_capacity(PALIKAT_WIDTH);
            for x in 0..PALIKAT_WIDTH {
                if self.cell_is_falling(x, y) && !self.game_over {
                    spans.push(Span::styled("▓▓", styles.accent.add_modifier(Modifier::BOLD)));
                } else if self.board[y][x] {
                    spans.push(Span::styled("██", styles.base.add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw("  "));
                }
            }
            lines.push(Line::from(spans));
        }
        let board = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .style(styles.base);
        frame.render_widget(board, board_area);

        let state = if self.game_over {
            "PELI PÄÄTTYI — Enter tai R aloittaa uudelleen"
        } else if self.paused {
            "TAUKO — P jatkaa"
        } else {
            "← → liikuta   ↑/Enter käännä   ↓ alas   välilyönti pudota"
        };
        let footer = Paragraph::new(format!("{state}\nEsc takaisin   P tauko   R uusi peli"))
            .alignment(Alignment::Center)
            .style(styles.muted)
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, rows[2]);
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
        let rows = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .split(outer);

        let title = Paragraph::new(vec![
            Line::from(Span::styled("MATO", styles.base.add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(format!("Pisteet {}", self.score), styles.muted)),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(title, rows[0]);

        let board_area = centered_size(rows[1], (MATO_WIDTH as u16) * 2 + 2, MATO_HEIGHT as u16 + 2);
        let head = self.snake.front().copied();
        let mut lines = Vec::with_capacity(MATO_HEIGHT as usize);
        for y in 0..MATO_HEIGHT {
            let mut spans = Vec::with_capacity(MATO_WIDTH as usize);
            for x in 0..MATO_WIDTH {
                let point = (x, y);
                if Some(point) == head {
                    spans.push(Span::styled("▓▓", styles.accent.add_modifier(Modifier::BOLD)));
                } else if self.snake.contains(&point) {
                    spans.push(Span::styled("██", styles.base.add_modifier(Modifier::BOLD)));
                } else if point == self.food {
                    spans.push(Span::styled("● ", styles.base.add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw("  "));
                }
            }
            lines.push(Line::from(spans));
        }
        let board = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL))
            .style(styles.base);
        frame.render_widget(board, board_area);

        let state = if self.game_over {
            "PELI PÄÄTTYI — Enter tai R aloittaa uudelleen"
        } else if self.paused {
            "TAUKO — P jatkaa"
        } else {
            "Nuolinäppäimet ohjaavat matoa"
        };
        let footer = Paragraph::new(format!("{state}\nEsc takaisin   P tauko   R uusi peli"))
            .alignment(Alignment::Center)
            .style(styles.muted)
            .wrap(Wrap { trim: true });
        frame.render_widget(footer, rows[2]);
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

fn inset(area: Rect, amount: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(amount.min(area.width / 2)),
        y: area.y.saturating_add(amount.min(area.height / 2)),
        width: area.width.saturating_sub(amount.min(area.width / 2).saturating_mul(2)),
        height: area.height.saturating_sub(amount.min(area.height / 2).saturating_mul(2)),
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
        // I
        (0, 0 | 2) => [(0, 1), (1, 1), (2, 1), (3, 1)],
        (0, 1 | 3) => [(2, 0), (2, 1), (2, 2), (2, 3)],
        // O
        (1, _) => [(1, 0), (2, 0), (1, 1), (2, 1)],
        // T
        (2, 0) => [(1, 0), (0, 1), (1, 1), (2, 1)],
        (2, 1) => [(1, 0), (1, 1), (2, 1), (1, 2)],
        (2, 2) => [(0, 1), (1, 1), (2, 1), (1, 2)],
        (2, 3) => [(1, 0), (0, 1), (1, 1), (1, 2)],
        // S
        (3, 0) => [(1, 0), (2, 0), (0, 1), (1, 1)],
        (3, 1) => [(1, 0), (1, 1), (2, 1), (2, 2)],
        (3, 2) => [(1, 1), (2, 1), (0, 2), (1, 2)],
        (3, 3) => [(0, 0), (0, 1), (1, 1), (1, 2)],
        // Z
        (4, 0) => [(0, 0), (1, 0), (1, 1), (2, 1)],
        (4, 1) => [(2, 0), (1, 1), (2, 1), (1, 2)],
        (4, 2) => [(0, 1), (1, 1), (1, 2), (2, 2)],
        (4, 3) => [(1, 0), (0, 1), (1, 1), (0, 2)],
        // J
        (5, 0) => [(0, 0), (0, 1), (1, 1), (2, 1)],
        (5, 1) => [(1, 0), (2, 0), (1, 1), (1, 2)],
        (5, 2) => [(0, 1), (1, 1), (2, 1), (2, 2)],
        (5, 3) => [(1, 0), (1, 1), (0, 2), (1, 2)],
        // L
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
        game.current = Piece { kind: 1, rotation: 0, x: -1, y: 0 };
        assert!(!game.try_move(-1, 0));
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
