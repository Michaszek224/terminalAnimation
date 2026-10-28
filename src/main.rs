mod math;
mod palette;
mod render;
mod shapes;

use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};

use math::{Mat3, Vec3};
use palette::ColorScheme;
use render::{RenderMode, Renderer};
use shapes::{Model, ShapeKind};

/// Animowane bryły 3D w terminalu. Bez flag kształt, tryb i kolory są losowane.
///
/// Klawisze: ←/→ lub p/n zmiana kształtu, m tryb, c kolory, r losuj wszystko,
/// spacja pauza, +/- prędkość, h pasek statusu, q/Esc wyjście.
#[derive(Parser, Debug)]
#[command(name = "shapes", version)]
struct Cli {
    /// Kształt do wyświetlenia (domyślnie losowy)
    #[arg(short, long, value_enum)]
    shape: Option<ShapeKind>,

    /// Styl renderowania (domyślnie losowy)
    #[arg(short, long, value_enum)]
    mode: Option<RenderMode>,

    /// Paleta kolorów (domyślnie losowa)
    #[arg(short, long, value_enum)]
    color: Option<ColorScheme>,

    /// Mnożnik prędkości obrotu
    #[arg(long, default_value_t = 1.0)]
    speed: f32,

    /// Wielkość bryły względem ekranu
    #[arg(long, default_value_t = 1.0)]
    size: f32,

    /// Limit klatek na sekundę
    #[arg(long, default_value_t = 30)]
    fps: u32,

    /// Co ile sekund losować nowy kształt
    #[arg(long, value_name = "SEKUNDY")]
    cycle: Option<f32>,

    /// Ukryj pasek statusu na dole ekranu
    #[arg(long)]
    no_hud: bool,

    /// Wypisz jedną klatkę na stdout i zakończ (bez trybu interaktywnego)
    #[arg(long)]
    once: bool,
}

/// Tiny xorshift generator; randomness here is purely cosmetic.
struct Rng(u64);

impl Rng {
    fn from_time() -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        Rng(nanos ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    fn pick_other<T: Copy + PartialEq>(&mut self, items: &[T], current: T) -> T {
        let others: Vec<T> = items.iter().copied().filter(|&v| v != current).collect();
        if others.is_empty() { current } else { self.pick(&others) }
    }
}

fn random_color(rng: &mut Rng) -> ColorScheme {
    let colored: Vec<ColorScheme> =
        ColorScheme::value_variants().iter().copied().filter(|&c| c != ColorScheme::Plain).collect();
    rng.pick(&colored)
}

/// Next/previous variant of a clap enum, wrapping around.
fn step<T: ValueEnum + PartialEq + Copy>(current: T, delta: isize) -> T {
    let all = T::value_variants();
    let i = all.iter().position(|&v| v == current).unwrap_or(0) as isize;
    all[(i + delta).rem_euclid(all.len() as isize) as usize]
}

fn name<T: ValueEnum>(v: T) -> String {
    v.to_possible_value().map(|p| p.get_name().to_string()).unwrap_or_default()
}

struct Scene {
    shape: ShapeKind,
    model: Model,
    mode: RenderMode,
    color: ColorScheme,
}

impl Scene {
    fn new(shape: ShapeKind, mode: RenderMode, color: ColorScheme) -> Self {
        Self { shape, model: shape.build(), mode, color }
    }

    fn set_shape(&mut self, shape: ShapeKind) {
        self.shape = shape;
        self.model = shape.build();
    }

    /// The tesseract has no solid faces, so it is always drawn as a wireframe.
    fn effective_mode(&self) -> RenderMode {
        match self.model {
            Model::Tesseract(_) => RenderMode::Wire,
            Model::Mesh(_) => self.mode,
        }
    }

    fn draw(&self, r: &mut Renderer, t: f32, size: f32) {
        let rot = Mat3::rotation(t * 0.7, t, t * 0.3);
        let rotate = |vs: &[Vec3]| vs.iter().map(|&v| rot * v).collect::<Vec<_>>();
        match &self.model {
            Model::Mesh(m) => match self.mode {
                RenderMode::Fill => r.fill(&rotate(&m.verts), &rotate(&m.normals), &m.tris, self.color, t, size),
                RenderMode::Wire => r.wire(&rotate(&m.verts), &m.edges, self.color, t, size),
            },
            Model::Tesseract(ts) => {
                let rot = Mat3::rotation(t * 0.25, t * 0.35, 0.0);
                let verts: Vec<Vec3> = ts.project(t * 0.8, t * 0.5).into_iter().map(|v| rot * v).collect();
                r.wire(&verts, &ts.edges, self.color, t, size);
            }
        }
    }

    fn status(&self, speed: f32, paused: bool) -> String {
        format!(
            " {} · {} · {} · {:.2}x{}  │  ←/→ kształt  m tryb  c kolor  r losuj  spacja pauza  +/- prędkość  h ukryj  q wyjście",
            name(self.shape),
            name(self.effective_mode()),
            name(self.color),
            speed,
            if paused { " · pauza" } else { "" },
        )
    }
}

/// Restores the terminal on drop, including when unwinding from a panic.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide, terminal::Clear(terminal::ClearType::All))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let mut rng = Rng::from_time();
    let shape = cli.shape.unwrap_or_else(|| rng.pick(ShapeKind::value_variants()));
    let mode = cli.mode.unwrap_or_else(|| rng.pick(RenderMode::value_variants()));
    let color = cli.color.unwrap_or_else(|| random_color(&mut rng));
    let scene = Scene::new(shape, mode, color);

    if cli.once {
        let (w, h) = terminal::size().unwrap_or((80, 24));
        let mut r = Renderer::new(w as usize, h.saturating_sub(1) as usize);
        scene.draw(&mut r, 0.8, cli.size);
        println!("{}", r.compose(scene.color != ColorScheme::Plain));
        return Ok(());
    }
    run(&cli, scene, rng)
}

enum Action {
    Quit,
    Continue,
}

fn handle_key(key: KeyEvent, scene: &mut Scene, rng: &mut Rng, r: &mut Renderer, speed: &mut f32, paused: &mut bool) -> Action {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Action::Quit,
        KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
        KeyCode::Char(' ') => *paused = !*paused,
        KeyCode::Right | KeyCode::Char('n') => scene.set_shape(step(scene.shape, 1)),
        KeyCode::Left | KeyCode::Char('p') => scene.set_shape(step(scene.shape, -1)),
        KeyCode::Char('m') => scene.mode = step(scene.mode, 1),
        KeyCode::Char('c') => scene.color = step(scene.color, 1),
        KeyCode::Char('r') => {
            scene.set_shape(rng.pick_other(ShapeKind::value_variants(), scene.shape));
            scene.mode = rng.pick(RenderMode::value_variants());
            scene.color = random_color(rng);
        }
        KeyCode::Char('+') | KeyCode::Char('=') => *speed = (*speed * 1.25).min(20.0),
        KeyCode::Char('-') | KeyCode::Char('_') => *speed = (*speed / 1.25).max(0.05),
        KeyCode::Char('h') => r.hud = !r.hud,
        _ => {}
    }
    Action::Continue
}

fn run(cli: &Cli, mut scene: Scene, mut rng: Rng) -> io::Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        default_hook(info);
    }));

    let _guard = TerminalGuard::enter()?;
    let mut stdout = io::stdout();
    let (w, h) = terminal::size()?;
    let mut renderer = Renderer::new(w as usize, h as usize);
    renderer.hud = !cli.no_hud;

    let frame = Duration::from_secs_f32(1.0 / cli.fps.max(1) as f32);
    let (mut t, mut speed, mut paused) = (0.0f32, cli.speed, false);
    let mut since_switch = 0.0f32;
    let mut last = Instant::now();

    loop {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;
        if !paused {
            t += dt * speed;
            since_switch += dt;
        }
        if let Some(period) = cli.cycle
            && since_switch >= period
        {
            scene.set_shape(rng.pick_other(ShapeKind::value_variants(), scene.shape));
            since_switch = 0.0;
        }

        scene.draw(&mut renderer, t, cli.size);
        renderer.status(&scene.status(speed, paused));
        stdout.write_all(b"\x1b[H")?;
        stdout.write_all(renderer.compose(scene.color != ColorScheme::Plain).as_bytes())?;
        stdout.flush()?;

        // Wait out the rest of the frame while staying responsive to input.
        let deadline = now + frame;
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if !event::poll(timeout)? {
                break;
            }
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Action::Quit = handle_key(key, &mut scene, &mut rng, &mut renderer, &mut speed, &mut paused) {
                        return Ok(());
                    }
                }
                Event::Resize(w, h) => {
                    renderer.resize(w as usize, h as usize);
                    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
                }
                _ => {}
            }
        }
    }
}
