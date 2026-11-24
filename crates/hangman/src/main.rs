mod settings;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::asset::RenderAssetUsages;
use bevy::camera::ScalingMode;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hangman_core::{entries, Difficulty, Game, Status, MAX_WRONG};

// ---------- palette ----------
const BG: Color = Color::srgb(0.09, 0.10, 0.13);
const WOOD: Color = Color::srgb(0.62, 0.45, 0.28);
const ROPE: Color = Color::srgb(0.80, 0.72, 0.55);
const FIGURE: Color = Color::srgb(0.93, 0.93, 0.95);
const TEXT: Color = Color::srgb(0.92, 0.93, 0.96);
const MUTED: Color = Color::srgb(0.60, 0.63, 0.70);
const KEY_IDLE: Color = Color::srgb(0.20, 0.22, 0.29);
const KEY_HIT: Color = Color::srgb(0.18, 0.55, 0.34);
const KEY_MISS: Color = Color::srgb(0.45, 0.18, 0.20);
const WIN: Color = Color::srgb(0.35, 0.85, 0.50);
const LOSE: Color = Color::srgb(0.95, 0.40, 0.40);
const ACCENT: Color = Color::srgb(0.95, 0.75, 0.35);
const HARD: Color = Color::srgb(0.95, 0.55, 0.25);

const KEY_ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

// ---------- resources & components ----------

/// The current game, wrapped so Bevy's change detection tells us when to redraw.
#[derive(Resource)]
struct Session {
    game: Game,
    difficulty: Difficulty,
    settings_open: bool,
    /// Counter mixed into the seed so two games started in the same instant still differ.
    games_started: u64,
}

impl Session {
    fn new() -> Self {
        let mut s = Self {
            game: Game::new("hangman"),
            // HANGMAN_MODE=easy|hard overrides the saved setting (handy for testing);
            // otherwise use what the player picked last time, defaulting to Easy.
            difficulty: std::env::var("HANGMAN_MODE")
                .ok()
                .and_then(|m| Difficulty::from_name(&m))
                .or_else(settings::load_difficulty)
                .unwrap_or_default(),
            settings_open: false,
            games_started: 0,
        };
        s.restart();
        // Handy for testing: HANGMAN_WORD=rust forces the first word.
        if let Ok(w) = std::env::var("HANGMAN_WORD") {
            let w = w.to_ascii_lowercase();
            if let Some(entry) = entries().find(|e| e.word == w) {
                s.game = Game::from_entry(entry);
            } else if !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic()) {
                s.game = Game::new(&w);
            }
        }
        s
    }

    fn restart(&mut self) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.games_started += 1;
        let previous = self.game.reveal();
        let mut seed = nanos ^ self.games_started.wrapping_mul(0x9E37_79B9);
        // Never serve the same word twice in a row.
        loop {
            let next = Game::from_seed(seed);
            if next.reveal() != previous {
                self.game = next;
                return;
            }
            seed = seed.wrapping_add(1);
        }
    }

    /// Choose a difficulty. A change starts a fresh game so hints can't be used to cheat.
    fn set_difficulty(&mut self, difficulty: Difficulty) {
        if self.difficulty != difficulty {
            self.difficulty = difficulty;
            settings::save_difficulty(difficulty);
            self.restart();
        }
    }

    fn toggle_difficulty(&mut self) {
        self.set_difficulty(self.difficulty.toggled());
    }
}

/// A gallows/figure piece that appears once `wrong_count >= 0` wrong guesses.
#[derive(Component)]
struct Part(usize);

/// Which piece of information a text entity shows.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Field {
    Word,
    Wrong,
    Lives,
    Status,
    Category,
    Clue,
}

/// The full-screen settings panel (hidden unless the menu is open).
#[derive(Component)]
struct SettingsOverlay;

/// The gear button that opens and closes the settings menu.
#[derive(Component)]
struct GearButton;

/// A selectable difficulty row inside the settings menu.
#[derive(Component)]
struct DifficultyOption(Difficulty);

#[derive(Component)]
struct LetterKey(char);

fn main() {
    App::new()
        .insert_resource(ClearColor(BG))
        .insert_resource(Session::new())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Hangman".into(),
                // Wayland app_id / X11 WM_CLASS: lets launchers match the window to hangman.desktop.
                name: Some("hangman".into()),
                resolution: (1000u32, 640u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_camera, spawn_gallows, spawn_ui))
        .add_systems(
            Update,
            (
                scale_ui_to_window,
                keyboard_input,
                refresh_view,
                refresh_settings,
            )
                .chain(),
        )
        .run();
}

/// Design size of the scene. The camera and UI scale so this area always fits the window.
const DESIGN_W: f32 = 1000.0;
const DESIGN_H: f32 = 640.0;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: DESIGN_W,
                min_height: DESIGN_H,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Keep UI the same visual size as the world-space gallows when the window is resized or tiled.
fn scale_ui_to_window(windows: Query<&Window>, mut ui_scale: ResMut<UiScale>) {
    if let Ok(w) = windows.single() {
        let scale = (w.width() / DESIGN_W).min(w.height() / DESIGN_H).max(0.1);
        if (ui_scale.0 - scale).abs() > f32::EPSILON {
            ui_scale.0 = scale;
        }
    }
}

// ---------- gallows ----------

fn spawn_gallows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let bar = |commands: &mut Commands,
               color: Color,
               size: Vec2,
               pos: Vec2,
               rot: f32,
               part: Option<usize>| {
        let mut e = commands.spawn((
            Sprite::from_color(color, size),
            Transform::from_xyz(pos.x, pos.y, 0.0).with_rotation(Quat::from_rotation_z(rot)),
        ));
        if let Some(n) = part {
            e.insert((Part(n), Visibility::Hidden));
        }
    };

    // Fixed gallows (always visible), in the left half of the window.
    let px = -167.0; // x of the rope / figure
    bar(
        &mut commands,
        WOOD,
        Vec2::new(280.0, 16.0),
        Vec2::new(-262.0, -230.0),
        0.0,
        None,
    ); // base
    bar(
        &mut commands,
        WOOD,
        Vec2::new(16.0, 430.0),
        Vec2::new(-350.0, -15.0),
        0.0,
        None,
    ); // pole
    bar(
        &mut commands,
        WOOD,
        Vec2::new(200.0, 16.0),
        Vec2::new(-257.0, 200.0),
        0.0,
        None,
    ); // beam
    bar(
        &mut commands,
        ROPE,
        Vec2::new(6.0, 50.0),
        Vec2::new(px, 167.0),
        0.0,
        None,
    ); // rope

    // Figure parts, revealed one per wrong guess.
    let head = meshes.add(Annulus::new(26.0, 32.0));
    commands.spawn((
        Mesh2d(head),
        MeshMaterial2d(materials.add(FIGURE)),
        Transform::from_xyz(px, 110.0, 0.0),
        Visibility::Hidden,
        Part(1),
    ));
    bar(
        &mut commands,
        FIGURE,
        Vec2::new(8.0, 100.0),
        Vec2::new(px, 28.0),
        0.0,
        Some(2),
    ); // body
    let arm = Vec2::new(8.0, 64.0);
    bar(
        &mut commands,
        FIGURE,
        arm,
        Vec2::new(px - 22.0, 42.0),
        -0.785,
        Some(3),
    ); // left arm
    bar(
        &mut commands,
        FIGURE,
        arm,
        Vec2::new(px + 22.0, 42.0),
        0.785,
        Some(4),
    ); // right arm
    let leg = Vec2::new(8.0, 72.0);
    bar(
        &mut commands,
        FIGURE,
        leg,
        Vec2::new(px - 18.0, -52.0),
        -0.52,
        Some(5),
    ); // left leg
    bar(
        &mut commands,
        FIGURE,
        leg,
        Vec2::new(px + 18.0, -52.0),
        0.52,
        Some(6),
    ); // right leg
}

// ---------- UI ----------

fn label(text: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(size),
        TextColor(color),
    )
}

fn spawn_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(14.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn(label("H A N G M A N", 26.0, MUTED));

            // Hint panel. Fixed height so the layout doesn't jump between modes.
            root.spawn(Node {
                height: Val::Px(54.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|hint| {
                hint.spawn((label("", 22.0, ACCENT), Field::Category));
                hint.spawn((label("", 18.0, MUTED), Field::Clue));
            });

            root.spawn((
                label("", 52.0, TEXT),
                Field::Word,
                // Fixed height so the layout doesn't jump when the font shrinks for long words.
                Node {
                    height: Val::Px(66.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ));
            root.spawn((label("", 22.0, MUTED), Field::Wrong));
            root.spawn((label("", 20.0, MUTED), Field::Lives));
            root.spawn((label("", 26.0, TEXT), Field::Status));

            // On-screen keyboard.
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            })
            .with_children(|kb| {
                for row in KEY_ROWS {
                    kb.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|r| {
                        for ch in row.chars() {
                            r.spawn((
                                Node {
                                    width: Val::Px(38.0),
                                    height: Val::Px(44.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(KEY_IDLE),
                                LetterKey(ch),
                            ))
                            .with_child(label(&ch.to_string(), 22.0, TEXT))
                            .observe(
                                |click: On<PointerClick>,
                                 keys: Query<&LetterKey>,
                                 mut session: ResMut<Session>| {
                                    if let Ok(key) = keys.get(click.entity) {
                                        session.game.guess(key.0);
                                    }
                                },
                            );
                        }
                    });
                }
            });

            // Buttons: new game and difficulty.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(11.0)),
                        ..default()
                    },
                    BackgroundColor(KEY_IDLE),
                ))
                .with_child(label("New game  (Enter)", 19.0, TEXT))
                .observe(|_: On<PointerClick>, mut session: ResMut<Session>| {
                    session.restart();
                });
            });
        });

    spawn_settings(&mut commands, &mut images);
}

/// Draw a gear icon into an RGBA image (white on transparent) so we don't depend on font glyphs.
fn gear_image() -> Image {
    const N: u32 = 96;
    const TEETH: f32 = 8.0;
    let c = (N as f32 - 1.0) / 2.0;
    let mut data = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let r = (dx * dx + dy * dy).sqrt();
            let turns = dy.atan2(dx) / std::f32::consts::TAU * TEETH;
            let in_tooth = turns.rem_euclid(1.0) < 0.5;
            let outer = if in_tooth { 46.0 } else { 36.0 };
            // 1px soft edges on the outline and on the centre hole.
            let alpha = ((outer - r + 0.5).clamp(0.0, 1.0)) * ((r - 15.0 + 0.5).clamp(0.0, 1.0));
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn spawn_settings(commands: &mut Commands, images: &mut Assets<Image>) {
    let gear = images.add(gear_image());

    // Gear button, top-right corner.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                right: Val::Px(14.0),
                width: Val::Px(46.0),
                height: Val::Px(46.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(KEY_IDLE),
            GlobalZIndex(5),
            GearButton,
        ))
        .with_child((
            Node {
                width: Val::Px(30.0),
                height: Val::Px(30.0),
                ..default()
            },
            ImageNode::new(gear).with_color(TEXT),
        ))
        .observe(|_: On<PointerClick>, mut session: ResMut<Session>| {
            session.settings_open = !session.settings_open;
        });

    // Settings overlay: dimmed backdrop with a centered panel. Hidden until opened.
    commands
        .spawn((
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(10),
            SettingsOverlay,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(420.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(26.0)),
                        row_gap: Val::Px(14.0),
                        border_radius: BorderRadius::all(Val::Px(14.0)),
                        ..default()
                    },
                    BackgroundColor(BG),
                ))
                .with_children(|panel| {
                    panel.spawn(label("Settings", 30.0, TEXT));
                    panel.spawn(label("Difficulty", 20.0, ACCENT));

                    let options = [
                        (Difficulty::Easy, "Shows a category and a short clue"),
                        (Difficulty::Hard, "No hints. Just the blanks"),
                    ];
                    for (difficulty, blurb) in options {
                        panel
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                                    row_gap: Val::Px(4.0),
                                    border_radius: BorderRadius::all(Val::Px(8.0)),
                                    ..default()
                                },
                                BackgroundColor(KEY_IDLE),
                                DifficultyOption(difficulty),
                            ))
                            .with_children(|opt| {
                                opt.spawn(label(difficulty.label(), 24.0, TEXT));
                                opt.spawn(label(blurb, 16.0, MUTED));
                            })
                            .observe(
                                |click: On<PointerClick>,
                                 options: Query<&DifficultyOption>,
                                 mut session: ResMut<Session>| {
                                    if let Ok(opt) = options.get(click.entity) {
                                        session.set_difficulty(opt.0);
                                    }
                                },
                            );
                    }

                    panel
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(11.0)),
                                margin: UiRect::top(Val::Px(6.0)),
                                align_self: AlignSelf::FlexEnd,
                                border_radius: BorderRadius::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(KEY_IDLE),
                        ))
                        .with_child(label("Close  (Esc)", 19.0, TEXT))
                        .observe(|_: On<PointerClick>, mut session: ResMut<Session>| {
                            session.settings_open = false;
                        });

                    // Quit is last: the one way out that works on every OS and window manager.
                    panel
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(18.0), Val::Px(11.0)),
                                margin: UiRect::top(Val::Px(6.0)),
                                align_self: AlignSelf::FlexEnd,
                                border_radius: BorderRadius::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(KEY_IDLE),
                        ))
                        .with_child(label(&format!("Quit  ({QUIT_HINT})"), 19.0, TEXT))
                        .observe(|_: On<PointerClick>, mut exit: MessageWriter<AppExit>| {
                            exit.write(AppExit::Success);
                        });
                });
        });
}

// ---------- systems ----------

/// Quit shortcut: Cmd+Q on macOS, Ctrl+Q elsewhere.
#[cfg(target_os = "macos")]
const QUIT_HINT: &str = "Cmd+Q";
#[cfg(not(target_os = "macos"))]
const QUIT_HINT: &str = "Ctrl+Q";

fn is_quit_modifier(ev: &KeyboardInput) -> bool {
    if cfg!(target_os = "macos") {
        matches!(ev.key_code, KeyCode::SuperLeft | KeyCode::SuperRight)
            || ev.logical_key == Key::Super
    } else {
        matches!(ev.key_code, KeyCode::ControlLeft | KeyCode::ControlRight)
            || ev.logical_key == Key::Control
    }
}

fn keyboard_input(
    mut events: MessageReader<KeyboardInput>,
    mut session: ResMut<Session>,
    mut exit: MessageWriter<AppExit>,
    mut modifier_down: Local<bool>,
) {
    for ev in events.read() {
        // Track the quit modifier from the ordered event stream, so a fast
        // press+release inside one frame is still seen in the right order.
        if is_quit_modifier(ev) {
            *modifier_down = ev.state == ButtonState::Pressed;
            continue;
        }
        if ev.state != ButtonState::Pressed {
            continue;
        }
        // Quit works everywhere, even with the menu open (checked before letter guessing).
        let is_q = ev.key_code == KeyCode::KeyQ
            || matches!(&ev.logical_key, Key::Character(c) if c.eq_ignore_ascii_case("q"));
        if is_q && *modifier_down {
            exit.write(AppExit::Success);
            continue;
        }
        // Esc opens/closes the settings menu from anywhere.
        if ev.logical_key == Key::Escape {
            session.settings_open = !session.settings_open;
            continue;
        }
        // While the menu is open the game ignores the keyboard.
        if session.settings_open {
            continue;
        }
        match &ev.logical_key {
            Key::Character(s) => {
                for c in s.chars() {
                    session.game.guess(c);
                }
            }
            // Enter starts a new game once this one is over, or on demand.
            Key::Enter => session.restart(),
            // Tab quickly flips Easy/Hard (and starts a new game).
            Key::Tab => session.toggle_difficulty(),
            _ => {}
        }
    }
}

/// Show/hide the settings menu and highlight the active difficulty.
fn refresh_settings(
    session: Res<Session>,
    mut overlay: Query<&mut Node, With<SettingsOverlay>>,
    mut options: Query<(&DifficultyOption, &mut BackgroundColor), Without<GearButton>>,
    mut gear: Query<&mut BackgroundColor, With<GearButton>>,
) {
    if !session.is_changed() {
        return;
    }
    for mut node in &mut overlay {
        node.display = if session.settings_open {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (opt, mut bg) in &mut options {
        bg.0 = if opt.0 == session.difficulty {
            KEY_HIT
        } else {
            KEY_IDLE
        };
    }
    for mut bg in &mut gear {
        bg.0 = if session.settings_open {
            KEY_HIT
        } else {
            KEY_IDLE
        };
    }
}

fn refresh_view(
    session: Res<Session>,
    mut parts: Query<(&Part, &mut Visibility)>,
    mut keys: Query<(&LetterKey, &mut BackgroundColor)>,
    mut fields: Query<(&Field, &mut Text, &mut TextFont, &mut TextColor)>,
) {
    if !session.is_changed() {
        return;
    }
    let g = &session.game;
    let wrong_count = g.wrong_count();
    let status = g.status();
    let hints = session.difficulty.shows_hints();

    for (part, mut vis) in &mut parts {
        *vis = if wrong_count >= part.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (key, mut bg) in &mut keys {
        bg.0 = if !g.is_guessed(key.0) {
            KEY_IDLE
        } else if g.is_hit(key.0) {
            KEY_HIT
        } else {
            KEY_MISS
        };
    }

    for (field, mut text, mut font, mut color) in &mut fields {
        match field {
            Field::Word => {
                // Shrink long words so they always fit the right-hand panel (~440 design px wide).
                let letters = g.reveal().len() as f32;
                font.font_size = FontSize::Px((340.0 / letters).min(52.0));
                // On a loss, show the full word instead of the blanks.
                text.0 = if status == Status::Lost {
                    g.reveal()
                        .chars()
                        .map(String::from)
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    g.masked()
                };
            }
            Field::Wrong => {
                let list: Vec<String> = g.wrong_letters().iter().map(|c| c.to_string()).collect();
                text.0 = if list.is_empty() {
                    "Wrong: none yet".to_string()
                } else {
                    format!("Wrong: {}", list.join(" "))
                };
            }
            Field::Lives => text.0 = format!("Guesses left: {}", MAX_WRONG - wrong_count),
            Field::Status => match status {
                Status::Playing => {
                    text.0 = "Type a letter or click one".to_string();
                    color.0 = MUTED;
                }
                Status::Won => {
                    text.0 = "You won!  Enter = new game".to_string();
                    color.0 = WIN;
                }
                Status::Lost => {
                    text.0 = "You lost.  Enter = new game".to_string();
                    color.0 = LOSE;
                }
            },
            Field::Category => {
                text.0 = if !hints {
                    "No hints".to_string()
                } else if g.category().is_empty() {
                    String::new()
                } else {
                    format!("Category: {}", g.category())
                };
                color.0 = if hints { ACCENT } else { HARD };
            }
            Field::Clue => {
                text.0 = if hints {
                    g.clue().to_string()
                } else {
                    "You're on your own!".to_string()
                };
            }
        }
    }
}
