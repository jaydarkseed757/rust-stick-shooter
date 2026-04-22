use macroquad::prelude::*;
use macroquad::rand::gen_range;
use gilrs::{Axis, Button, EventType, Gilrs};

const PLAYER_SPEED: f32 = 220.0;
const PLAYER_RADIUS: f32 = 10.0;
const BULLET_SPEED: f32 = 450.0;
const BULLET_LIFETIME: f32 = 1.5;
const SHOOT_COOLDOWN: f32 = 0.1;
const INVINCIBLE_TIME: f32 = 2.5;
const ARENA_MARGIN: f32 = 28.0;

// ── Gamepad state (polled once per frame in main) ─────────────────────────────

#[derive(Default, Clone, Copy)]
struct GamepadState {
    left_stick: Vec2,   // movement, deadzone already applied
    right_stick: Vec2,  // aim/shoot, deadzone already applied
    confirm: bool,      // Start / South button just-pressed this frame
}

// ── Title-screen stars ────────────────────────────────────────────────────────

struct Star {
    x: f32,
    y: f32,
    speed: f32,
    radius: f32,
    brightness: f32,
}

fn init_stars() -> Vec<Star> {
    let m = ARENA_MARGIN;
    let w = screen_width();
    let h = screen_height();
    let mut v = Vec::with_capacity(120);
    for _ in 0..60 {
        v.push(Star { x: gen_range(m, w - m), y: gen_range(m, h - m),
            speed: gen_range(15.0f32, 35.0), radius: gen_range(0.4f32, 0.9),
            brightness: gen_range(0.15f32, 0.35) });
    }
    for _ in 0..35 {
        v.push(Star { x: gen_range(m, w - m), y: gen_range(m, h - m),
            speed: gen_range(55.0f32, 90.0), radius: gen_range(0.8f32, 1.4),
            brightness: gen_range(0.4f32, 0.65) });
    }
    for _ in 0..15 {
        v.push(Star { x: gen_range(m, w - m), y: gen_range(m, h - m),
            speed: gen_range(110.0f32, 170.0), radius: gen_range(1.4f32, 2.4),
            brightness: gen_range(0.7f32, 1.0) });
    }
    v
}

// ── 3-D wireframe ship (title screen) ────────────────────────────────────────

fn draw_3d_ship(scr_cx: f32, scr_cy: f32, angle: f32) {
    // Ship vertices (x = right, y = down, z = toward viewer).
    let verts: [(f32, f32, f32); 8] = [
        (  0.0, -1.0,  20.0),  // 0 nose
        (-16.0,  0.0,  -6.0),  // 1 left wing tip
        ( 16.0,  0.0,  -6.0),  // 2 right wing tip
        (  0.0,  0.0, -12.0),  // 3 tail
        (  0.0, -6.0,   7.0),  // 4 cockpit
        ( -4.5,  2.5,  -9.0),  // 5 left engine
        (  4.5,  2.5,  -9.0),  // 6 right engine
        (  0.0,  0.0,   1.0),  // 7 wing join
    ];
    let edges: [(usize, usize); 11] = [
        (0, 1), (0, 2), (0, 4),          // nose struts
        (1, 3), (2, 3),                   // wings to tail
        (4, 7), (7, 1), (7, 2),           // cockpit frame
        (3, 5), (3, 6), (5, 6),           // tail / engines
    ];

    // Primary Y-axis spin with a gentle X-axis wobble for depth.
    let (sin_ay, cos_ay) = angle.sin_cos();
    let ax = (angle * 0.35).sin() * 0.22;
    let (sin_ax, cos_ax) = ax.sin_cos();

    let transform = |(x, y, z): (f32, f32, f32)| -> (f32, f32, f32) {
        let x1 =  x * cos_ay + z * sin_ay;
        let z1 = -x * sin_ay + z * cos_ay;
        let y2 = y * cos_ax - z1 * sin_ax;
        let z2 = y * sin_ax + z1 * cos_ax;
        (x1, y2, z2)
    };
    // Perspective: camera at z = +88, focal = 480.
    let project = |(x, y, z): (f32, f32, f32)| -> Vec2 {
        let s = 480.0 / (88.0 - z).max(20.0);
        vec2(scr_cx + x * s, scr_cy + y * s)
    };

    let t: Vec<(f32, f32, f32)> = verts.iter().map(|&v| transform(v)).collect();
    let p: Vec<Vec2>             = t.iter()   .map(|&v| project(v))  .collect();

    // Wide glow pass first, then crisp edges on top.
    for &(a, b) in &edges {
        draw_line(p[a].x, p[a].y, p[b].x, p[b].y, 5.0,
                  Color::new(0.05, 0.3, 0.7, 0.22));
    }
    for &(a, b) in &edges {
        let mid_z  = (t[a].2 + t[b].2) * 0.5;
        let bright = ((mid_z + 15.0) / 40.0).clamp(0.35, 1.0);
        let c = Color::new(bright * 0.35, bright * 0.85, bright, 1.0);
        draw_line(p[a].x, p[a].y, p[b].x, p[b].y, 2.0, c);
    }
    // Engine glow dots
    for idx in [5usize, 6] {
        draw_circle(p[idx].x, p[idx].y, 5.5, Color::new(0.2, 0.5, 1.0, 0.9));
        draw_circle(p[idx].x, p[idx].y, 9.5, Color::new(0.1, 0.3, 0.9, 0.22));
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn circles_overlap(a: Vec2, ra: f32, b: Vec2, rb: f32) -> bool {
    (a - b).length_squared() < (ra + rb) * (ra + rb)
}

fn random_edge_pos() -> Vec2 {
    let m = ARENA_MARGIN + 16.0;
    let w = screen_width();
    let h = screen_height();
    match gen_range(0u8, 4) {
        0 => vec2(gen_range(m, w - m), m),
        1 => vec2(gen_range(m, w - m), h - m),
        2 => vec2(m, gen_range(m, h - m)),
        _ => vec2(w - m, gen_range(m, h - m)),
    }
}

fn spawn_particles(particles: &mut Vec<Particle>, pos: Vec2, color: Color, n: usize) {
    for _ in 0..n {
        particles.push(Particle::new(pos, color));
    }
}

// ── Player ───────────────────────────────────────────────────────────────────

struct Player {
    pos: Vec2,
    lives: i32,
    invincible_timer: f32,
    shoot_cooldown: f32,
    angle: f32, // facing direction in radians, toward mouse
}

impl Player {
    fn new() -> Self {
        Player {
            pos: vec2(screen_width() * 0.5, screen_height() * 0.5),
            lives: 3,
            invincible_timer: 0.0,
            shoot_cooldown: 0.0,
            angle: 0.0,
        }
    }

    fn update(&mut self, dt: f32, gp: &GamepadState) {
        // Movement: WASD + left gamepad stick
        let mut dir = Vec2::ZERO;
        if is_key_down(KeyCode::W) { dir.y -= 1.0; }
        if is_key_down(KeyCode::S) { dir.y += 1.0; }
        if is_key_down(KeyCode::A) { dir.x -= 1.0; }
        if is_key_down(KeyCode::D) { dir.x += 1.0; }
        if gp.left_stick != Vec2::ZERO { dir += gp.left_stick; }
        if dir != Vec2::ZERO { dir = dir.normalize(); }
        self.pos += dir * PLAYER_SPEED * dt;

        let m = ARENA_MARGIN + PLAYER_RADIUS;
        self.pos.x = self.pos.x.clamp(m, screen_width() - m);
        self.pos.y = self.pos.y.clamp(m, screen_height() - m);

        // Facing angle: right stick takes priority, then mouse
        if gp.right_stick != Vec2::ZERO {
            self.angle = gp.right_stick.y.atan2(gp.right_stick.x);
        } else {
            let mp = mouse_position();
            let diff = vec2(mp.0, mp.1) - self.pos;
            if diff.length_squared() > 25.0 {
                self.angle = diff.y.atan2(diff.x);
            }
        }

        if self.invincible_timer > 0.0 { self.invincible_timer -= dt; }
        if self.shoot_cooldown > 0.0   { self.shoot_cooldown -= dt; }
    }

    // Returns normalized fire direction, or None if not shooting this frame.
    fn shoot_dir(&self, gp: &GamepadState) -> Option<Vec2> {
        // Right gamepad stick: auto-fires while pushed past deadzone
        if gp.right_stick != Vec2::ZERO {
            return Some(gp.right_stick.normalize());
        }
        // Arrow keys (digital, fires while held)
        let mut arrow = Vec2::ZERO;
        if is_key_down(KeyCode::Up)    { arrow.y -= 1.0; }
        if is_key_down(KeyCode::Down)  { arrow.y += 1.0; }
        if is_key_down(KeyCode::Left)  { arrow.x -= 1.0; }
        if is_key_down(KeyCode::Right) { arrow.x += 1.0; }
        if arrow != Vec2::ZERO {
            return Some(arrow.normalize());
        }
        // Mouse aim: click or hold Space
        if is_mouse_button_down(MouseButton::Left) || is_key_down(KeyCode::Space) {
            let mp = mouse_position();
            let diff = vec2(mp.0, mp.1) - self.pos;
            if diff.length_squared() > 25.0 {
                return Some(diff.normalize());
            }
        }
        None
    }

    fn draw(&self) {
        // Flicker while invincible
        if self.invincible_timer > 0.0 && (self.invincible_timer * 10.0) as i32 % 2 == 0 {
            return;
        }
        let tip  = self.pos + vec2(self.angle.cos(), self.angle.sin()) * 12.0;
        let left = self.pos + vec2((self.angle + 2.3).cos(), (self.angle + 2.3).sin()) * 9.0;
        let rght = self.pos + vec2((self.angle - 2.3).cos(), (self.angle - 2.3).sin()) * 9.0;
        draw_triangle(tip, left, rght, SKYBLUE);
        draw_line(tip.x, tip.y, left.x, left.y, 1.5, WHITE);
        draw_line(left.x, left.y, rght.x, rght.y, 1.5, WHITE);
        draw_line(rght.x, rght.y, tip.x, tip.y, 1.5, WHITE);
        // Engine glow at rear
        let back = self.pos - vec2(self.angle.cos(), self.angle.sin()) * 7.0;
        draw_circle(back.x, back.y, 3.0, Color::new(0.1, 0.6, 1.0, 0.9));
    }
}

// ── Enemy ────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum EnemyKind { Grunt, Spheroid, Tank }

struct Enemy {
    pos: Vec2,
    vel: Vec2,
    kind: EnemyKind,
    health: i32,
    angle: f32,       // facing / visual rotation
    shoot_timer: f32, // Tank: time until next shot
    spawn_timer: f32, // Spheroid: time until next grunt spawn
    dead: bool,
}

impl Enemy {
    fn new_grunt(pos: Vec2) -> Self {
        Enemy { pos, vel: Vec2::ZERO, kind: EnemyKind::Grunt, health: 1,
                angle: 0.0, shoot_timer: 0.0, spawn_timer: 0.0, dead: false }
    }
    fn new_spheroid(pos: Vec2) -> Self {
        let a = gen_range(0.0f32, std::f32::consts::TAU);
        Enemy { pos, vel: vec2(a.cos(), a.sin()) * 62.0, kind: EnemyKind::Spheroid, health: 2,
                angle: 0.0, shoot_timer: 0.0, spawn_timer: 4.0, dead: false }
    }
    fn new_tank(pos: Vec2) -> Self {
        Enemy { pos, vel: Vec2::ZERO, kind: EnemyKind::Tank, health: 3,
                angle: 0.0, shoot_timer: 2.0, spawn_timer: 0.0, dead: false }
    }

    fn radius(&self) -> f32 {
        match self.kind { EnemyKind::Grunt => 9.0, EnemyKind::Spheroid => 14.0, EnemyKind::Tank => 12.0 }
    }
    fn score_value(&self) -> u32 {
        match self.kind { EnemyKind::Grunt => 100, EnemyKind::Spheroid => 500, EnemyKind::Tank => 250 }
    }
    fn particle_color(&self) -> Color {
        match self.kind { EnemyKind::Grunt => RED, EnemyKind::Spheroid => PURPLE, EnemyKind::Tank => GREEN }
    }

    // Returns Some(fire_dir) when this enemy fires a bullet this frame.
    fn update(&mut self, player_pos: Vec2, dt: f32, wave: u32) -> Option<Vec2> {
        match self.kind {
            EnemyKind::Grunt => {
                let dir = (player_pos - self.pos).normalize_or_zero();
                self.vel = dir * (80.0 + wave as f32 * 4.0);
                self.angle = dir.y.atan2(dir.x);
            }
            EnemyKind::Spheroid => {
                // Arc motion: slowly rotate the velocity vector
                let rot = 0.9 * dt;
                let (s, c) = rot.sin_cos();
                let vx = self.vel.x * c - self.vel.y * s;
                let vy = self.vel.x * s + self.vel.y * c;
                self.vel = vec2(vx, vy);
                self.angle += dt * 2.5;
                self.spawn_timer -= dt;
            }
            EnemyKind::Tank => {
                let dir = (player_pos - self.pos).normalize_or_zero();
                self.vel = dir * 38.0;
                self.angle = dir.y.atan2(dir.x);
                self.shoot_timer -= dt;
                if self.shoot_timer <= 0.0 {
                    self.shoot_timer = (2.0 - wave as f32 * 0.1).max(0.7);
                    // pos update happens below before we return
                    self.pos += self.vel * dt;
                    let m = ARENA_MARGIN + self.radius();
                    self.pos.x = self.pos.x.clamp(m, screen_width() - m);
                    self.pos.y = self.pos.y.clamp(m, screen_height() - m);
                    return Some(dir);
                }
            }
        }

        self.pos += self.vel * dt;

        let m = ARENA_MARGIN + self.radius();
        if self.kind == EnemyKind::Spheroid {
            // Bounce off arena walls
            if self.pos.x <= m { self.pos.x = m; self.vel.x = self.vel.x.abs(); }
            if self.pos.x >= screen_width() - m { self.pos.x = screen_width() - m; self.vel.x = -self.vel.x.abs(); }
            if self.pos.y <= m { self.pos.y = m; self.vel.y = self.vel.y.abs(); }
            if self.pos.y >= screen_height() - m { self.pos.y = screen_height() - m; self.vel.y = -self.vel.y.abs(); }
        } else {
            self.pos.x = self.pos.x.clamp(m, screen_width() - m);
            self.pos.y = self.pos.y.clamp(m, screen_height() - m);
        }
        None
    }

    fn draw(&self) {
        match self.kind {
            EnemyKind::Grunt => {
                let tip  = self.pos + vec2(self.angle.cos(), self.angle.sin()) * 11.0;
                let left = self.pos + vec2((self.angle + 2.3).cos(), (self.angle + 2.3).sin()) * 8.0;
                let rght = self.pos + vec2((self.angle - 2.3).cos(), (self.angle - 2.3).sin()) * 8.0;
                draw_triangle(tip, left, rght, Color::new(0.85, 0.15, 0.05, 1.0));
                draw_line(tip.x, tip.y, left.x, left.y, 1.2, Color::new(1.0, 0.5, 0.1, 1.0));
                draw_line(left.x, left.y, rght.x, rght.y, 1.2, Color::new(1.0, 0.5, 0.1, 1.0));
                draw_line(rght.x, rght.y, tip.x, tip.y, 1.2, Color::new(1.0, 0.5, 0.1, 1.0));
            }
            EnemyKind::Spheroid => {
                draw_circle_lines(self.pos.x, self.pos.y, 14.0, 1.5, PURPLE);
                draw_circle(self.pos.x, self.pos.y, 4.5, PURPLE);
                // Spinning equator line
                let ax = self.pos.x + self.angle.cos() * 13.0;
                let ay = self.pos.y + self.angle.sin() * 13.0;
                let bx = self.pos.x - self.angle.cos() * 13.0;
                let by = self.pos.y - self.angle.sin() * 13.0;
                draw_line(ax, ay, bx, by, 1.5, Color::new(0.8, 0.2, 0.9, 0.7));
            }
            EnemyKind::Tank => {
                draw_poly(self.pos.x, self.pos.y, 6, 12.0, self.angle.to_degrees(), DARKGREEN);
                draw_poly_lines(self.pos.x, self.pos.y, 6, 12.0, self.angle.to_degrees(), 1.5, GREEN);
                // Gun barrel pointing at player
                let bx = self.pos.x + self.angle.cos() * 18.0;
                let by = self.pos.y + self.angle.sin() * 18.0;
                draw_line(self.pos.x, self.pos.y, bx, by, 3.5, GREEN);
            }
        }
    }
}

// ── Bullet ───────────────────────────────────────────────────────────────────

struct Bullet {
    pos: Vec2,
    vel: Vec2,
    from_player: bool,
    lifetime: f32,
    dead: bool,
}

impl Bullet {
    fn new(pos: Vec2, dir: Vec2, from_player: bool) -> Self {
        Bullet { pos, vel: dir * BULLET_SPEED, from_player, lifetime: BULLET_LIFETIME, dead: false }
    }

    fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.lifetime -= dt;
        if self.lifetime <= 0.0
            || self.pos.x < ARENA_MARGIN || self.pos.x > screen_width()  - ARENA_MARGIN
            || self.pos.y < ARENA_MARGIN || self.pos.y > screen_height() - ARENA_MARGIN
        {
            self.dead = true;
        }
    }

    fn draw(&self) {
        if self.from_player {
            draw_circle(self.pos.x, self.pos.y, 3.5, YELLOW);
            draw_circle(self.pos.x, self.pos.y, 5.5, Color::new(1.0, 1.0, 0.0, 0.15));
        } else {
            draw_circle(self.pos.x, self.pos.y, 3.0, ORANGE);
        }
    }
}

// ── Particle ─────────────────────────────────────────────────────────────────

struct Particle {
    pos: Vec2,
    vel: Vec2,
    lifetime: f32,
    max_lifetime: f32,
    color: Color,
    seg_len: f32,
    angle: f32,
}

impl Particle {
    fn new(pos: Vec2, color: Color) -> Self {
        let angle = gen_range(0.0f32, std::f32::consts::TAU);
        let lt    = gen_range(0.35f32, 0.85);
        Particle {
            pos,
            vel: vec2(angle.cos(), angle.sin()) * gen_range(60.0f32, 190.0),
            lifetime: lt,
            max_lifetime: lt,
            color,
            seg_len: gen_range(4.0f32, 11.0),
            angle,
        }
    }

    fn update(&mut self, dt: f32) {
        self.pos += self.vel * dt;
        self.vel *= 1.0 - dt * 3.5; // drag
        self.lifetime -= dt;
    }

    fn draw(&self) {
        let alpha = (self.lifetime / self.max_lifetime).clamp(0.0, 1.0);
        let c = Color::new(self.color.r, self.color.g, self.color.b, alpha);
        let ex = self.pos.x + self.angle.cos() * self.seg_len;
        let ey = self.pos.y + self.angle.sin() * self.seg_len;
        draw_line(self.pos.x, self.pos.y, ex, ey, 2.0, c);
    }
}

// ── Wave config ───────────────────────────────────────────────────────────────

fn wave_enemies(wave: u32) -> (usize, usize, usize) {
    let grunts    = (4 + wave * 3) as usize;
    let spheroids = if wave >= 2 { ((wave - 1) / 2) as usize } else { 0 };
    let tanks     = if wave >= 3 { ((wave - 2) / 3) as usize } else { 0 };
    (grunts, spheroids, tanks)
}

fn spawn_wave(enemies: &mut Vec<Enemy>, wave: u32, player_pos: Vec2) {
    let (g, s, t) = wave_enemies(wave);
    let safe_sq = 120.0f32 * 120.0;

    let mut try_add = |mk: fn(Vec2) -> Enemy| {
        for _ in 0..10 {
            let pos = random_edge_pos();
            if (pos - player_pos).length_squared() > safe_sq {
                enemies.push(mk(pos));
                return;
            }
        }
        enemies.push(mk(random_edge_pos()));
    };

    for _ in 0..g { try_add(Enemy::new_grunt); }
    for _ in 0..s { try_add(Enemy::new_spheroid); }
    for _ in 0..t { try_add(Enemy::new_tank); }
}

// ── Leaderboard ───────────────────────────────────────────────────────────────

const SCORES_FILE: &str = "scores.dat";

#[derive(Clone)]
struct ScoreEntry { initials: String, score: u32 }

struct Leaderboard { entries: Vec<ScoreEntry> }

impl Leaderboard {
    fn defaults() -> Vec<ScoreEntry> {
        vec![
            ScoreEntry { initials: "ACE".into(), score: 52000 },
            ScoreEntry { initials: "REX".into(), score: 43500 },
            ScoreEntry { initials: "ZAX".into(), score: 37200 },
            ScoreEntry { initials: "KAI".into(), score: 31800 },
            ScoreEntry { initials: "MAX".into(), score: 27400 },
            ScoreEntry { initials: "JAX".into(), score: 23100 },
            ScoreEntry { initials: "LEX".into(), score: 19600 },
            ScoreEntry { initials: "VEX".into(), score: 16300 },
            ScoreEntry { initials: "DUO".into(), score: 13000 },
            ScoreEntry { initials: "HAL".into(), score: 10100 },
        ]
    }

    fn load() -> Self {
        let text = std::fs::read_to_string(SCORES_FILE).unwrap_or_default();
        let mut entries: Vec<ScoreEntry> = text
            .lines()
            .filter_map(|line| {
                let mut p = line.split_whitespace();
                let initials = p.next()?.to_string();
                let score: u32 = p.next()?.parse().ok()?;
                Some(ScoreEntry { initials, score })
            })
            .collect();
        entries.sort_by(|a, b| b.score.cmp(&a.score));
        if entries.len() < 10 { entries = Self::defaults(); }
        entries.truncate(10);
        Leaderboard { entries }
    }

    fn save(&self) {
        let s: String = self.entries.iter()
            .map(|e| format!("{} {}\n", e.initials, e.score))
            .collect();
        let _ = std::fs::write(SCORES_FILE, s);
    }

    fn top_score(&self) -> u32 {
        self.entries.first().map_or(0, |e| e.score)
    }

    fn qualifies(&self, score: u32) -> bool {
        score > self.entries.last().map_or(0, |e| e.score)
    }

    fn rank_of(&self, score: u32) -> usize {
        self.entries.iter().position(|e| score > e.score).unwrap_or(self.entries.len())
    }

    fn insert(&mut self, initials: String, score: u32) -> usize {
        let rank = self.rank_of(score);
        self.entries.insert(rank, ScoreEntry { initials, score });
        self.entries.truncate(10);
        self.save();
        rank
    }
}

// ── Game ──────────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Screen { Menu, Playing, GameOver, EnterInitials, Leaderboard }

struct Game {
    screen: Screen,
    player: Player,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    particles: Vec<Particle>,
    score: u32,
    wave: u32,
    wave_banner_timer: f32,
    next_wave_timer: f32,
    // title screen / attract mode
    stars: Vec<Star>,
    ship_angle: f32,
    attract_timer: f32,  // 0-30 = title, 30-45 = scores, resets at 45
    // leaderboard
    leaderboard: Leaderboard,
    initials_input: String,
    new_entry_rank: Option<usize>,
}

impl Game {
    fn new() -> Self {
        Game {
            screen: Screen::Menu,
            player: Player::new(),
            enemies: Vec::new(),
            bullets: Vec::new(),
            particles: Vec::new(),
            score: 0,
            wave: 1,
            wave_banner_timer: 0.0,
            next_wave_timer: 0.0,
            stars: Vec::new(),
            ship_angle: 0.0,
            attract_timer: 0.0,
            leaderboard: Leaderboard::load(),
            initials_input: String::new(),
            new_entry_rank: None,
        }
    }

    fn start(&mut self) {
        self.player = Player::new();
        self.enemies.clear();
        self.bullets.clear();
        self.particles.clear();
        self.score = 0;
        self.wave = 1;
        self.wave_banner_timer = 2.0;
        self.next_wave_timer = 2.0;
        self.screen = Screen::Playing;
    }

    fn update_menu(&mut self, dt: f32) {
        if self.stars.is_empty() {
            self.stars = init_stars();
        }
        let m = ARENA_MARGIN;
        let w = screen_width();
        let h = screen_height();
        for star in &mut self.stars {
            star.x -= star.speed * dt;
            if star.x < m {
                star.x = w - m;
                star.y = gen_range(m, h - m);
            }
        }
        self.ship_angle += dt * 0.75;
        self.attract_timer += dt;
        if self.attract_timer >= 45.0 { self.attract_timer = 0.0; }
    }

    fn update(&mut self, dt: f32, gp: &GamepadState) {
        let confirm = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) || gp.confirm;
        match self.screen {
            Screen::Menu => {
                self.update_menu(dt);
                if confirm { self.start(); }
            }
            Screen::Playing => self.update_game(dt, gp),
            Screen::GameOver => {
                for p in &mut self.particles { p.update(dt); }
                self.particles.retain(|p| p.lifetime > 0.0);
                if confirm {
                    if self.leaderboard.qualifies(self.score) {
                        self.initials_input.clear();
                        self.new_entry_rank = None;
                        self.screen = Screen::EnterInitials;
                    } else {
                        self.new_entry_rank = None;
                        self.screen = Screen::Leaderboard;
                    }
                }
            }
            Screen::EnterInitials => {
                for p in &mut self.particles { p.update(dt); }
                self.particles.retain(|p| p.lifetime > 0.0);
                self.update_enter_initials();
            }
            Screen::Leaderboard => {
                for p in &mut self.particles { p.update(dt); }
                self.particles.retain(|p| p.lifetime > 0.0);
                if confirm {
                    self.attract_timer = 0.0; // restart attract cycle
                    self.new_entry_rank = None;
                    self.screen = Screen::Menu;
                }
            }
        }
    }

    fn update_enter_initials(&mut self) {
        while let Some(ch) = get_char_pressed() {
            if self.initials_input.len() < 3 && ch.is_ascii_alphabetic() {
                self.initials_input.push(ch.to_ascii_uppercase());
            }
        }
        if is_key_pressed(KeyCode::Backspace) && !self.initials_input.is_empty() {
            self.initials_input.pop();
        }
        if is_key_pressed(KeyCode::Enter) && self.initials_input.len() == 3 {
            let rank = self.leaderboard.insert(self.initials_input.clone(), self.score);
            self.new_entry_rank = Some(rank);
            self.screen = Screen::Leaderboard;
        }
    }

    fn update_game(&mut self, dt: f32, gp: &GamepadState) {
        // Wave spawn timing
        if self.next_wave_timer > 0.0 {
            self.next_wave_timer -= dt;
            if self.next_wave_timer <= 0.0 {
                spawn_wave(&mut self.enemies, self.wave, self.player.pos);
            }
        }
        if self.wave_banner_timer > 0.0 { self.wave_banner_timer -= dt; }

        self.player.update(dt, gp);

        // Player shooting
        if self.player.shoot_cooldown <= 0.0 {
            if let Some(dir) = self.player.shoot_dir(gp) {
                self.bullets.push(Bullet::new(self.player.pos, dir, true));
                self.player.shoot_cooldown = SHOOT_COOLDOWN;
            }
        }

        // Update enemies; collect new spawns and enemy shots separately
        let mut new_bullets: Vec<Bullet> = Vec::new();
        let mut grunt_spawn_positions: Vec<Vec2> = Vec::new();
        let player_pos = self.player.pos;
        let wave = self.wave;

        for e in &mut self.enemies {
            if let Some(fire_dir) = e.update(player_pos, dt, wave) {
                new_bullets.push(Bullet::new(e.pos, fire_dir, false));
            }
            if e.kind == EnemyKind::Spheroid && e.spawn_timer <= 0.0 {
                e.spawn_timer = 4.0;
                grunt_spawn_positions.push(e.pos);
            }
        }
        for pos in grunt_spawn_positions { self.enemies.push(Enemy::new_grunt(pos)); }
        self.bullets.extend(new_bullets);

        for b in &mut self.bullets   { b.update(dt); }
        for p in &mut self.particles { p.update(dt); }
        self.particles.retain(|p| p.lifetime > 0.0);

        // Collision: player bullets vs enemies
        struct Kill { pos: Vec2, color: Color, score: u32 }
        let mut kills: Vec<Kill> = Vec::new();

        'outer: for b in &mut self.bullets {
            if !b.from_player || b.dead { continue; }
            for e in &mut self.enemies {
                if e.dead { continue; }
                if circles_overlap(b.pos, 4.0, e.pos, e.radius()) {
                    b.dead = true;
                    e.health -= 1;
                    if e.health <= 0 {
                        e.dead = true;
                        kills.push(Kill { pos: e.pos, color: e.particle_color(), score: e.score_value() });
                    }
                    continue 'outer;
                }
            }
        }
        for k in kills {
            self.score += k.score;
            spawn_particles(&mut self.particles, k.pos, k.color, 10);
        }

        // Collision: enemy bullets / contact vs player
        let mut hit = false;
        if self.player.invincible_timer <= 0.0 {
            for b in &mut self.bullets {
                if b.from_player || b.dead { continue; }
                if circles_overlap(b.pos, 4.0, self.player.pos, PLAYER_RADIUS) {
                    b.dead = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                for e in &self.enemies {
                    if e.dead { continue; }
                    if circles_overlap(e.pos, e.radius(), self.player.pos, PLAYER_RADIUS) {
                        hit = true;
                        break;
                    }
                }
            }
        }
        if hit { self.hit_player(); }

        self.bullets.retain(|b| !b.dead);
        self.enemies.retain(|e| !e.dead);

        // Advance wave when arena is cleared
        if self.enemies.is_empty() && self.next_wave_timer <= 0.0 {
            self.wave += 1;
            self.wave_banner_timer = 2.5;
            self.next_wave_timer = 2.5;
        }
    }

    fn hit_player(&mut self) {
        spawn_particles(&mut self.particles, self.player.pos, SKYBLUE, 14);
        self.player.invincible_timer = INVINCIBLE_TIME;
        self.player.lives -= 1;
        if self.player.lives <= 0 {
            self.screen = Screen::GameOver;
        }
    }

    fn draw(&self) {
        clear_background(BLACK);
        match self.screen {
            Screen::Menu => if self.attract_timer >= 30.0 {
                self.draw_attract_leaderboard();
            } else {
                self.draw_menu();
            },
            Screen::Playing      => self.draw_game(),
            Screen::GameOver     => self.draw_gameover(),
            Screen::EnterInitials => self.draw_enter_initials(),
            Screen::Leaderboard  => self.draw_leaderboard(),
        }
    }

    fn draw_arena(&self) {
        let m = ARENA_MARGIN;
        let w = screen_width();
        let h = screen_height();
        // Outer glow
        draw_rectangle_lines(m - 4.0, m - 4.0, w - (m - 4.0) * 2.0, h - (m - 4.0) * 2.0,
                             1.5, Color::new(0.0, 0.4, 0.8, 0.3));
        // Main border
        draw_rectangle_lines(m, m, w - m * 2.0, h - m * 2.0,
                             2.0, Color::new(0.1, 0.6, 1.0, 0.85));
    }

    fn draw_hud(&self) {
        let score_str = format!("SCORE: {:06}", self.score);
        draw_text(&score_str, ARENA_MARGIN + 5.0, ARENA_MARGIN - 7.0, 22.0, WHITE);

        let hs_str = format!("BEST: {:06}", self.leaderboard.top_score());
        let tw = measure_text(&hs_str, None, 22, 1.0).width;
        draw_text(&hs_str, screen_width() * 0.5 - tw * 0.5, ARENA_MARGIN - 7.0, 22.0, DARKGRAY);

        // Lives as small player-ship triangles
        for i in 0..self.player.lives.max(0) {
            let x = screen_width() - ARENA_MARGIN - 10.0 - i as f32 * 22.0;
            let y = ARENA_MARGIN - 12.0;
            let tip  = vec2(x, y);
            let left = vec2(x - 6.0, y + 11.0);
            let rght = vec2(x + 6.0, y + 11.0);
            draw_triangle(tip, left, rght, SKYBLUE);
        }
    }

    fn draw_game(&self) {
        self.draw_arena();
        for p in &self.particles { p.draw(); }
        for b in &self.bullets   { b.draw(); }
        for e in &self.enemies   { e.draw(); }
        self.player.draw();
        self.draw_hud();

        if self.wave_banner_timer > 0.0 {
            let alpha = (self.wave_banner_timer * 1.5).min(1.0);
            let text = format!("WAVE {}", self.wave);
            let tw = measure_text(&text, None, 60, 1.0).width;
            draw_text(&text,
                      screen_width() * 0.5 - tw * 0.5,
                      screen_height() * 0.5,
                      60.0, Color::new(1.0, 1.0, 0.0, alpha));
        }
    }

    fn draw_menu(&self) {
        // ── parallax star field ──
        for star in &self.stars {
            let b = star.brightness;
            draw_circle(star.x, star.y, star.radius,
                        Color::new(b * 0.85, b * 0.92, b, 1.0));
        }

        self.draw_arena();

        let cx = screen_width() * 0.5;
        let cy = screen_height() * 0.5;

        // ── rotating 3-D ship (centred slightly above mid) ──
        draw_3d_ship(cx, cy - 10.0, self.ship_angle);

        // ── text layout pushed above/below the ship ──
        let title = "VECTOR STORM";
        let tw = measure_text(title, None, 72, 1.0).width;
        draw_text(title, cx - tw * 0.5, cy - 118.0, 72.0, YELLOW);

        let sub = "TWIN-STICK SHOOTER";
        let sw = measure_text(sub, None, 22, 1.0).width;
        draw_text(sub, cx - sw * 0.5, cy - 83.0, 22.0, DARKGRAY);

        let ctrl = "WASD: Move     Mouse/Click or Arrows: Shoot";
        let cw = measure_text(ctrl, None, 17, 1.0).width;
        draw_text(ctrl, cx - cw * 0.5, cy + 72.0, 17.0, Color::new(0.4, 0.4, 0.4, 1.0));

        let pulse = ((get_time() * 2.0).sin() as f32 * 0.35 + 0.65).max(0.0);
        let prompt = "PRESS ENTER OR SPACE TO START";
        let pw = measure_text(prompt, None, 26, 1.0).width;
        draw_text(prompt, cx - pw * 0.5, cy + 108.0, 26.0,
                  Color::new(0.2, pulse, 1.0, 1.0));

        let hs = format!("TOP SCORE: {:06}", self.leaderboard.top_score());
        let hw = measure_text(&hs, None, 20, 1.0).width;
        draw_text(&hs, cx - hw * 0.5, cy + 142.0, 20.0, WHITE);
    }

    fn draw_attract_leaderboard(&self) {
        // Stars still scroll in the background
        for star in &self.stars {
            let b = star.brightness;
            draw_circle(star.x, star.y, star.radius, Color::new(b * 0.85, b * 0.92, b, 1.0));
        }
        self.draw_arena();

        let cx  = screen_width()  * 0.5;
        let m   = ARENA_MARGIN;

        let title = "HIGH SCORES";
        let tw = measure_text(title, None, 48, 1.0).width;
        draw_text(title, cx - tw * 0.5, m + 42.0, 48.0, YELLOW);

        let entry_y0 = m + 88.0;
        let spacing  = (screen_height() - m - 50.0 - entry_y0) / 10.0;

        for (i, entry) in self.leaderboard.entries.iter().enumerate() {
            let y = entry_y0 + i as f32 * spacing;
            let color = if i == 0 { WHITE } else { GRAY };

            let rank_str = format!("{}.", i + 1);
            let rw = measure_text(&rank_str, None, 20, 1.0).width;
            draw_text(&rank_str, cx - 120.0 - rw, y, 20.0, color);
            draw_text(&entry.initials, cx - 100.0, y, 20.0, color);
            let score_str = format!("{:06}", entry.score);
            let ssw = measure_text(&score_str, None, 20, 1.0).width;
            draw_text(&score_str, cx + 80.0 - ssw * 0.5, y, 20.0, color);
        }

        let secs_left = (45.0 - self.attract_timer).ceil() as i32;
        let prompt = format!("PRESS ENTER TO PLAY  ({})", secs_left);
        let pw = measure_text(&prompt, None, 20, 1.0).width;
        draw_text(&prompt, cx - pw * 0.5, screen_height() - m - 8.0, 20.0,
                  Color::new(0.5, 0.5, 0.5, 1.0));
    }

    fn draw_gameover(&self) {
        self.draw_arena();
        for p in &self.particles { p.draw(); }

        let cx = screen_width() * 0.5;
        let cy = screen_height() * 0.5;

        let title = "GAME OVER";
        let tw = measure_text(title, None, 72, 1.0).width;
        draw_text(title, cx - tw * 0.5, cy - 55.0, 72.0, RED);

        let score_str = format!("SCORE: {:06}", self.score);
        let sw = measure_text(&score_str, None, 36, 1.0).width;
        draw_text(&score_str, cx - sw * 0.5, cy, 36.0, WHITE);

        let sub = if self.leaderboard.qualifies(self.score) {
            let rank = self.leaderboard.rank_of(self.score) + 1;
            format!("RANK #{} — ENTER YOUR INITIALS", rank)
        } else {
            "PRESS ENTER TO SEE HIGH SCORES".into()
        };
        let subw = measure_text(&sub, None, 20, 1.0).width;
        draw_text(&sub, cx - subw * 0.5, cy + 38.0, 20.0, YELLOW);

        let prompt = "PRESS ENTER OR SPACE TO CONTINUE";
        let pw = measure_text(prompt, None, 20, 1.0).width;
        draw_text(prompt, cx - pw * 0.5, cy + 80.0, 20.0, GRAY);
    }

    fn draw_enter_initials(&self) {
        self.draw_arena();
        for p in &self.particles { p.draw(); }

        let cx = screen_width() * 0.5;
        let cy = screen_height() * 0.5;
        let rank = self.leaderboard.rank_of(self.score) + 1;

        if rank == 1 {
            let t = "NEW HIGH SCORE!";
            let tw = measure_text(t, None, 36, 1.0).width;
            draw_text(t, cx - tw * 0.5, cy - 120.0, 36.0, YELLOW);
        }

        let rt = format!("RANK #{}", rank);
        let rw = measure_text(&rt, None, 28, 1.0).width;
        draw_text(&rt, cx - rw * 0.5, cy - 82.0, 28.0, WHITE);

        let st = format!("SCORE: {:06}", self.score);
        let sw = measure_text(&st, None, 22, 1.0).width;
        draw_text(&st, cx - sw * 0.5, cy - 50.0, 22.0, GRAY);

        let prompt = "ENTER YOUR INITIALS";
        let pw = measure_text(prompt, None, 20, 1.0).width;
        draw_text(prompt, cx - pw * 0.5, cy - 10.0, 20.0, DARKGRAY);

        // Three character slots
        let chars: Vec<char> = self.initials_input.chars().collect();
        let blink = (get_time() * 3.0) as i32 % 2 == 0;
        let slot_w = 38.0;
        let gap = 12.0;
        let total = 3.0 * slot_w + 2.0 * gap;
        let sx = cx - total * 0.5;
        let sy = cy + 18.0;

        for i in 0..3 {
            let x = sx + i as f32 * (slot_w + gap);
            let is_cursor = i == chars.len() && blink;
            let border_col = if i < chars.len() { WHITE }
                             else if is_cursor { SKYBLUE }
                             else { DARKGRAY };
            draw_rectangle_lines(x, sy, slot_w, slot_w + 8.0, 2.0, border_col);
            if i < chars.len() {
                let ch = chars[i].to_string();
                let cw = measure_text(&ch, None, 44, 1.0).width;
                draw_text(&ch, x + slot_w * 0.5 - cw * 0.5, sy + slot_w - 2.0, 44.0, WHITE);
            } else if is_cursor {
                draw_rectangle(x + 6.0, sy + slot_w - 4.0, slot_w - 12.0, 3.0, SKYBLUE);
            }
        }

        let hint = if self.initials_input.len() < 3 {
            "TYPE 3 LETTERS"
        } else {
            "PRESS ENTER TO CONFIRM"
        };
        let hw = measure_text(hint, None, 17, 1.0).width;
        draw_text(hint, cx - hw * 0.5, sy + 78.0, 17.0, DARKGRAY);
    }

    fn draw_leaderboard(&self) {
        self.draw_arena();
        for p in &self.particles { p.draw(); }

        let cx = screen_width() * 0.5;
        let m = ARENA_MARGIN;

        let title = "HIGH SCORES";
        let tw = measure_text(title, None, 40, 1.0).width;
        draw_text(title, cx - tw * 0.5, m + 36.0, 40.0, YELLOW);

        let st = format!("YOUR SCORE: {:06}", self.score);
        let sw = measure_text(&st, None, 20, 1.0).width;
        draw_text(&st, cx - sw * 0.5, m + 66.0, 20.0, WHITE);

        // Entries — three columns: rank, initials, score
        let entry_y0 = m + 100.0;
        let spacing  = (screen_height() - m - 50.0 - entry_y0) / 10.0;

        for (i, entry) in self.leaderboard.entries.iter().enumerate() {
            let y = entry_y0 + i as f32 * spacing;
            let is_new = self.new_entry_rank == Some(i);
            let color = if is_new { YELLOW } else if i == 0 { WHITE } else { GRAY };

            let rank_str = format!("{}.", i + 1);
            let rw = measure_text(&rank_str, None, 20, 1.0).width;
            draw_text(&rank_str, cx - 120.0 - rw, y, 20.0, color);
            draw_text(&entry.initials, cx - 100.0, y, 20.0, color);
            let score_str = format!("{:06}", entry.score);
            let ssw = measure_text(&score_str, None, 20, 1.0).width;
            draw_text(&score_str, cx + 80.0 - ssw * 0.5, y, 20.0, color);

            if is_new {
                draw_text("◄", cx + 90.0, y, 18.0, YELLOW);
            }
        }

        let prompt = "PRESS ENTER TO CONTINUE";
        let pw = measure_text(prompt, None, 20, 1.0).width;
        draw_text(prompt, cx - pw * 0.5, screen_height() - m - 8.0, 20.0, GRAY);
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[macroquad::main("Vector Storm")]
async fn main() {
    let mut game = Game::new();
    let mut gilrs = Gilrs::new().ok();
    let mut active_gamepad: Option<gilrs::GamepadId> = None;

    loop {
        // Poll gamepad events; track which pad is active and catch button presses.
        let mut gp = GamepadState::default();
        if let Some(ref mut g) = gilrs {
            while let Some(gilrs::Event { id, event, .. }) = g.next_event() {
                active_gamepad = Some(id);
                if let EventType::ButtonPressed(btn, _) = event {
                    if matches!(btn, Button::Start | Button::Select | Button::South) {
                        gp.confirm = true;
                    }
                }
            }
            if let Some(id) = active_gamepad {
                const DZ: f32 = 0.18;
                let pad = g.gamepad(id);
                let lx =  pad.axis_data(Axis::LeftStickX).map_or(0.0, |a| a.value());
                let ly = -pad.axis_data(Axis::LeftStickY).map_or(0.0, |a| a.value()); // flip Y
                let rx =  pad.axis_data(Axis::RightStickX).map_or(0.0, |a| a.value());
                let ry = -pad.axis_data(Axis::RightStickY).map_or(0.0, |a| a.value()); // flip Y
                let ls = vec2(lx, ly);
                let rs = vec2(rx, ry);
                if ls.length() > DZ { gp.left_stick  = ls; }
                if rs.length() > DZ { gp.right_stick = rs; }
            }
        }

        let dt = get_frame_time().min(0.05);
        game.update(dt, &gp);
        game.draw();
        next_frame().await;
    }
}
