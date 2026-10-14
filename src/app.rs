//! Web entry point and event handling, ported from the native `main.rs`.
//!
//! Window creation happens inside `resumed`, but `wgpu` initialization is
//! asynchronous on the web, so the renderer is built by a `spawn_local` task and
//! picked up on the next redraw. Benchmark, smoke, and CLI handling from the
//! native binary are intentionally not ported.
use crate::{
    engine::{
        audio::{Audio, Sound},
        text::TextMesh,
        window::WindowRenderer,
    },
    persistence::{BindingAction, BindingKey, GameSettings},
    physics::Aabb,
    platform::{Duration, Instant},
    simulation::{
        combat::Combat,
        game_mode::SessionMode,
        player::{FootstepClock, Input},
        time::FixedStep,
        weapon::{FireInput, Loadout, ShotSpreadRng, WeaponKind},
        Player,
    },
    ui::menu::{PauseMenuAction, PauseSelection, TitleMenuAction, TitleSelection},
    world, Scene,
};
use glam::Vec3;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

type RendererSlot = Rc<RefCell<Option<Result<WindowRenderer, String>>>>;

/// Initial canvas size. The web build stretches the canvas to the viewport, so
/// this only seeds the first frame before the browser reports its real size.
const DEFAULT_WINDOW_SIZE: (u32, u32) = (960, 540);

fn get_canvas() -> Option<web_sys::HtmlCanvasElement> {
    web_sys::window()?
        .document()?
        .get_element_by_id("game-canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()
}

fn webgpu_available() -> bool {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &wasm_bindgen::JsValue::from_str("navigator"));
    navigator
        .ok()
        .and_then(|navigator| {
            js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("gpu")).ok()
        })
        .is_some_and(|gpu| !gpu.is_undefined() && !gpu.is_null())
}

fn show_fallback() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(element) = document.get_element_by_id("fallback") {
        let _ = element.set_attribute("style", "display: flex;");
    }
    if let Some(canvas) = document.get_element_by_id("game-canvas") {
        let _ = canvas.set_attribute("style", "display: none;");
    }
    set_loading(false);
}

/// Shows or hides the full-screen loading overlay.
fn set_loading(visible: bool) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("loading"))
    {
        let display = if visible { "flex" } else { "none" };
        let _ = element.set_attribute("style", &format!("display: {display};"));
    }
}

/// Replaces the loading label with an error message and keeps it visible.
fn set_loading_error(message: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("loading-text"))
    {
        element.set_text_content(Some(message));
    }
    let spinner = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("loading"))
        .and_then(|element| element.query_selector(".spinner").ok().flatten());
    if let Some(spinner) = spinner {
        let _ = spinner.set_attribute("style", "display: none;");
    }
    set_loading(true);
}

fn console_log(message: &str) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(message));
}

fn console_error(message: &str) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(message));
}

fn binding_key_to_code(key: BindingKey) -> KeyCode {
    match key {
        BindingKey::A => KeyCode::KeyA,
        BindingKey::B => KeyCode::KeyB,
        BindingKey::C => KeyCode::KeyC,
        BindingKey::D => KeyCode::KeyD,
        BindingKey::E => KeyCode::KeyE,
        BindingKey::F => KeyCode::KeyF,
        BindingKey::G => KeyCode::KeyG,
        BindingKey::H => KeyCode::KeyH,
        BindingKey::I => KeyCode::KeyI,
        BindingKey::J => KeyCode::KeyJ,
        BindingKey::K => KeyCode::KeyK,
        BindingKey::L => KeyCode::KeyL,
        BindingKey::M => KeyCode::KeyM,
        BindingKey::N => KeyCode::KeyN,
        BindingKey::O => KeyCode::KeyO,
        BindingKey::P => KeyCode::KeyP,
        BindingKey::Q => KeyCode::KeyQ,
        BindingKey::R => KeyCode::KeyR,
        BindingKey::S => KeyCode::KeyS,
        BindingKey::T => KeyCode::KeyT,
        BindingKey::U => KeyCode::KeyU,
        BindingKey::V => KeyCode::KeyV,
        BindingKey::W => KeyCode::KeyW,
        BindingKey::X => KeyCode::KeyX,
        BindingKey::Y => KeyCode::KeyY,
        BindingKey::Z => KeyCode::KeyZ,
        BindingKey::Space => KeyCode::Space,
        BindingKey::LeftShift => KeyCode::ShiftLeft,
        BindingKey::RightShift => KeyCode::ShiftRight,
        BindingKey::Escape => KeyCode::Escape,
        BindingKey::Digit1 => KeyCode::Digit1,
        BindingKey::Digit2 => KeyCode::Digit2,
        BindingKey::Digit3 => KeyCode::Digit3,
    }
}

fn code_to_binding_key(code: KeyCode) -> Option<BindingKey> {
    Some(match code {
        KeyCode::KeyA => BindingKey::A,
        KeyCode::KeyB => BindingKey::B,
        KeyCode::KeyC => BindingKey::C,
        KeyCode::KeyD => BindingKey::D,
        KeyCode::KeyE => BindingKey::E,
        KeyCode::KeyF => BindingKey::F,
        KeyCode::KeyG => BindingKey::G,
        KeyCode::KeyH => BindingKey::H,
        KeyCode::KeyI => BindingKey::I,
        KeyCode::KeyJ => BindingKey::J,
        KeyCode::KeyK => BindingKey::K,
        KeyCode::KeyL => BindingKey::L,
        KeyCode::KeyM => BindingKey::M,
        KeyCode::KeyN => BindingKey::N,
        KeyCode::KeyO => BindingKey::O,
        KeyCode::KeyP => BindingKey::P,
        KeyCode::KeyQ => BindingKey::Q,
        KeyCode::KeyR => BindingKey::R,
        KeyCode::KeyS => BindingKey::S,
        KeyCode::KeyT => BindingKey::T,
        KeyCode::KeyU => BindingKey::U,
        KeyCode::KeyV => BindingKey::V,
        KeyCode::KeyW => BindingKey::W,
        KeyCode::KeyX => BindingKey::X,
        KeyCode::KeyY => BindingKey::Y,
        KeyCode::KeyZ => BindingKey::Z,
        KeyCode::Space => BindingKey::Space,
        KeyCode::ShiftLeft => BindingKey::LeftShift,
        KeyCode::ShiftRight => BindingKey::RightShift,
        KeyCode::Escape => BindingKey::Escape,
        KeyCode::Digit1 => BindingKey::Digit1,
        KeyCode::Digit2 => BindingKey::Digit2,
        KeyCode::Digit3 => BindingKey::Digit3,
        _ => return None,
    })
}

struct App {
    renderer: Option<WindowRenderer>,
    pending: Option<RendererSlot>,
    world_mesh: Scene,
    hud_mesh: Scene,
    text_mesh: TextMesh,
    clock: Instant,
    player: Player,
    colliders: Vec<Aabb>,
    input: Input,
    footstep_clock: FootstepClock,
    weapons: Loadout,
    shot_spread_rng: ShotSpreadRng,
    fire_input: FireInput,
    combat: Combat,
    settings: GameSettings,
    tracking_bot_count: u32,
    session: SessionMode,
    title_screen: bool,
    settings_menu: bool,
    settings_from_title: bool,
    title_selection: TitleSelection,
    pause_selection: PauseSelection,
    binding_selection: usize,
    waiting_for_binding: bool,
    audio: Option<Audio>,
    title: String,
    capture_requested: bool,
    captured: bool,
    window_focused: bool,
    fixed: FixedStep,
    next_frame_deadline: Option<Instant>,
    occluded: bool,
    suspended: bool,
    error: Option<String>,
    notice: Option<(String, Instant)>,
}

impl App {
    fn switch_weapon(&mut self, kind: WeaponKind) {
        if self.weapons.switch(kind) {
            self.fire_input.discard_press();
        }
    }
    fn fire(&mut self, held: bool, pressed: bool) {
        let kind = self.weapons.active_kind();
        let view_direction = self.player.aim_direction();
        let damage = self.weapons.active().data.damage;
        let recoil_kick = self.weapons.active().data.recoil_kick;
        if !self
            .weapons
            .fire_with_infinite_ammo(held, pressed, self.infinite_ammo_active())
        {
            return;
        }
        let weapon = self.weapons.active();
        let mut shot_direction = weapon.shot_direction(view_direction);
        if self.session.hip_fire_spread_enabled() && weapon.ads_progress <= 0.0 {
            shot_direction = self
                .shot_spread_rng
                .apply(shot_direction, weapon.data.hip_fire_spread_radians);
        }
        let (aspect, weapon_y_offset) = self
            .renderer
            .as_ref()
            .map(|renderer| {
                let (width, height) = renderer.internal_size();
                let (_, offset) = self.weapons.presentation();
                (
                    width as f32 / height.max(1) as f32,
                    offset + self.player.dash_weapon_offset(),
                )
            })
            .unwrap_or((16.0 / 9.0, 0.0));
        let fov = self.weapons.fov_with_base(self.settings.fov);
        let muzzle = self.weapons.active().muzzle_position(
            self.player.position,
            view_direction,
            aspect,
            fov,
            weapon_y_offset,
        );
        let shot = self.combat.shoot_from_muzzle_with_aim(
            self.player.position,
            muzzle,
            view_direction,
            shot_direction,
            &self.colliders,
            damage,
        );
        if self.session.recoil_enabled() {
            self.player.apply_recoil(recoil_kick);
        } else {
            self.weapons.active_mut().visual_recoil = 0.0;
        }
        if let Some(audio) = &self.audio {
            audio.play(match kind {
                WeaponKind::AssaultRifle => Sound::Fire,
                WeaponKind::Pistol => Sound::Pistol,
                WeaponKind::Sniper => Sound::Sniper,
            });
            if shot.killed {
                audio.play(Sound::Kill);
            } else if shot.headshot {
                audio.play(Sound::Head);
            } else if shot.hit {
                audio.play(Sound::Hit);
            }
        }
    }
    fn finish(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.exit();
    }
    fn adjust_runtime_setting(&mut self, setting: usize, direction: f32) {
        if setting == 3 && !self.settings_from_title && self.session == SessionMode::TrackingAim {
            self.tracking_bot_count =
                (self.tracking_bot_count as f32 + direction.signum()).clamp(1.0, 5.0) as u32;
        } else {
            self.settings.adjust_setting(setting, direction);
        }
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        console_error(&message);
        self.error = Some(message);
        self.finish(event_loop);
    }
    fn set_notice(&mut self, message: &str, detail: &str) {
        if detail.is_empty() {
            console_error(message);
        } else {
            console_error(&format!("{message}: {detail}"));
        }
        self.notice = Some((message.to_owned(), Instant::now()));
    }
    fn save_settings(&mut self) {
        match self.settings.save() {
            Ok(()) => self.notice = None,
            Err(error) => self.set_notice("SETTINGS NOT SAVED - CHECK STORAGE", &error),
        }
    }
    /// Retries capture when requested. Expected to run after focus is settled.
    fn try_capture(&mut self) {
        if !self.capture_requested || self.captured {
            return;
        }
        let Some(r) = &self.renderer else {
            return;
        };
        let result = r
            .window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| r.window.set_cursor_grab(CursorGrabMode::Confined));
        match result {
            Ok(()) => {
                r.window.set_cursor_visible(false);
                self.captured = true;
            }
            Err(e) => console_error(&format!("cursor grab failed: {e}")),
        }
    }
    fn release_capture(&mut self) {
        if let Some(r) = &self.renderer {
            let _ = r.window.set_cursor_grab(CursorGrabMode::None);
            r.window.set_cursor_visible(true);
        }
        self.captured = false;
        self.input = Input::default();
        self.fire_input.clear();
        self.fixed = FixedStep::new(60, 5);
        self.clock = Instant::now();
    }
    fn resume_from_pause(&mut self) {
        self.save_settings();
        self.settings_menu = false;
        if self.settings_from_title {
            self.settings_from_title = false;
            self.title_screen = true;
            self.capture_requested = false;
            self.release_capture();
            return;
        }
        self.capture_requested = true;
        self.try_capture();
    }
    fn change_mode(&mut self) {
        self.save_settings();
        self.settings_menu = false;
        self.settings_from_title = false;
        self.title_screen = true;
        self.title_selection = TitleSelection::default();
        self.capture_requested = false;
        self.release_capture();
    }
    fn begin_session(&mut self, mode: SessionMode) {
        self.player = Player::new();
        self.weapons = Loadout::default();
        self.fire_input = FireInput::default();
        self.footstep_clock.reset();
        self.combat = Combat::default();
        if mode == SessionMode::TrackingAim {
            self.tracking_bot_count = self.settings.bot_count.min(5);
            self.combat.configure_tracking_bots(
                self.tracking_bot_count,
                self.settings.bot_hp,
                self.settings.bot_speed,
                self.settings.bot_respawn_seconds,
                self.player.position,
            );
            self.colliders = world::tracking_colliders();
            if let Some(renderer) = &mut self.renderer {
                if let Err(error) = renderer.update_scene(&world::tracking_range()) {
                    self.error = Some(error);
                }
            }
        } else {
            let arena_scale = world::arena_scale_for_bot_count(self.settings.bot_count);
            self.combat.configure_bots_with_arena_scale(
                self.settings.bot_count,
                self.settings.bot_behavior,
                self.settings.bot_hp,
                self.settings.bot_speed,
                self.settings.bot_respawn_seconds,
                arena_scale,
            );
            self.colliders = world::arena_colliders_scaled(arena_scale);
            if let Some(renderer) = &mut self.renderer {
                if let Err(error) = renderer.update_scene(&world::arena_scaled(arena_scale)) {
                    self.error = Some(error);
                }
            }
        }
        self.session = mode;
        self.title_screen = false;
        self.settings_menu = false;
        self.settings_from_title = false;
        self.capture_requested = true;
        self.try_capture();
        self.clock = Instant::now();
    }
    fn infinite_ammo_active(&self) -> bool {
        self.session
            .infinite_ammo_enabled(self.settings.infinite_ammo)
    }
    fn apply_runtime_setting(&mut self, setting: usize) {
        match setting {
            3 | 4 | 6 | 7 | 8 => {
                if !self.settings_from_title && self.session == SessionMode::TrackingAim {
                    self.combat.configure_tracking_bots(
                        self.tracking_bot_count,
                        self.settings.bot_hp,
                        self.settings.bot_speed,
                        self.settings.bot_respawn_seconds,
                        self.player.position,
                    );
                    self.colliders = world::tracking_colliders();
                    if let Some(renderer) = &mut self.renderer {
                        if let Err(error) = renderer.update_scene(&world::tracking_range()) {
                            self.error = Some(error);
                        }
                    }
                } else {
                    let arena_scale = world::arena_scale_for_bot_count(self.settings.bot_count);
                    self.combat.configure_bots_with_arena_scale(
                        self.settings.bot_count,
                        self.settings.bot_behavior,
                        self.settings.bot_hp,
                        self.settings.bot_speed,
                        self.settings.bot_respawn_seconds,
                        arena_scale,
                    );
                    self.colliders = world::arena_colliders_scaled(arena_scale);
                    if let Some(renderer) = &mut self.renderer {
                        if let Err(error) = renderer.update_scene(&world::arena_scaled(arena_scale))
                        {
                            self.error = Some(error);
                        }
                    }
                }
            }
            5 => {
                if let Some(audio) = &mut self.audio {
                    audio.set_volume(self.settings.volume);
                }
            }
            9 => {
                if let Some(renderer) = &mut self.renderer {
                    let size = renderer.window.inner_size();
                    if let Err(error) =
                        renderer.resize_follow_display_aspect(size, self.settings.render_scale())
                    {
                        self.error = Some(error);
                    }
                }
            }
            10 => self.next_frame_deadline = None,
            _ => {}
        }
    }
    fn apply_fullscreen(&mut self) {
        if let Some(renderer) = &self.renderer {
            let fullscreen = self
                .settings
                .fullscreen
                .then(|| Fullscreen::Borderless(renderer.window.current_monitor()));
            renderer.window.set_fullscreen(fullscreen);
            if !self.settings.fullscreen {
                let (width, height) = DEFAULT_WINDOW_SIZE;
                let _ = renderer
                    .window
                    .request_inner_size(LogicalSize::new(width as f64, height as f64));
            }
        }
    }
    fn toggle_setting(&mut self, setting: usize) {
        self.settings.toggle_setting(setting);
        if setting == 1 {
            self.apply_fullscreen();
        }
        self.save_settings();
    }
    fn update_title(&mut self) {
        if let Some(r) = &self.renderer {
            let (w, h) = r.internal_size();
            let title = format!(
                "liteweight-fps | {w}×{h} | {} {}/{} | Bots {}/{} | Kills {} | 1:Pistol 2:AR 3:Sniper LMB:Fire RMB:ADS R:Reload Esc:Settings",
                self.weapons.active().kind.label(), self.weapons.active().ammo, self.weapons.active().data.magazine,
                self.combat.targets.iter().filter(|bot| bot.alive).count(), self.combat.targets.len(), self.combat.kills
            );
            if title != self.title {
                r.window.set_title(&title);
                self.title = title;
            }
        }
    }
    /// Picks up the asynchronously built renderer once it is ready.
    fn ensure_renderer(&mut self) -> bool {
        if self.renderer.is_some() {
            return true;
        }
        let Some(slot) = self.pending.clone() else {
            return false;
        };
        let taken = slot.borrow_mut().take();
        match taken {
            None => false,
            Some(Err(error)) => {
                self.pending = None;
                console_error(&format!("renderer init failed: {error}"));
                set_loading_error(&format!("初期化に失敗しました: {error}"));
                self.error = Some(error);
                false
            }
            Some(Ok(mut renderer)) => {
                self.pending = None;
                console_log(&format!(
                    "{}; {}",
                    renderer.adapter_info, renderer.presentation_info
                ));
                let size = renderer.window.inner_size();
                if let Err(error) =
                    renderer.resize_follow_display_aspect(size, self.settings.render_scale())
                {
                    console_error(&error);
                    self.error = Some(error);
                }
                renderer.prepare_text();
                self.renderer = Some(renderer);
                match Audio::new() {
                    Ok(mut audio) => {
                        audio.set_volume(self.settings.volume);
                        self.audio = Some(audio);
                    }
                    Err(e) => self.set_notice("AUDIO UNAVAILABLE - GAME WILL BE SILENT", &e),
                }
                self.update_title();
                set_loading(false);
                true
            }
        }
    }
    fn start_renderer_init(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let (window_width, window_height) = DEFAULT_WINDOW_SIZE;
        let mut attributes = Window::default_attributes()
            .with_title("liteweight-fps")
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(
                f64::from(window_width),
                f64::from(window_height),
            ));
        let canvas = get_canvas();
        attributes = attributes.with_canvas(canvas.clone());
        if canvas.is_none() {
            attributes = attributes.with_append(true);
        }
        if self.settings.fullscreen {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                self.fail(event_loop, e.to_string());
                return false;
            }
        };
        let mesh = world::arena_scaled(world::arena_scale_for_bot_count(self.settings.bot_count));
        let internal = (1280, 720);
        let slot: RendererSlot = Rc::new(RefCell::new(None));
        self.pending = Some(slot.clone());
        let redraw_window = window.clone();
        spawn_local(async move {
            let result = WindowRenderer::new(window, &mesh, internal).await;
            *slot.borrow_mut() = Some(result);
            redraw_window.request_redraw();
        });
        true
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        self.clock = Instant::now();
        if let Some(renderer) = &mut self.renderer {
            let size = renderer.window.inner_size();
            if let Err(e) =
                renderer.resize_follow_display_aspect(size, self.settings.render_scale())
            {
                self.fail(event_loop, e);
            }
            return;
        }
        if self.pending.is_some() {
            return;
        }
        self.start_renderer_init(event_loop);
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        self.release_capture();
        if let Some(r) = &mut self.renderer {
            let _ = r.resize(PhysicalSize::new(0, 0));
        }
    }

    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if self.captured {
                self.player.look(
                    delta.0 as f32
                        * 0.002
                        * self
                            .settings
                            .look_multiplier(self.weapons.active().ads_progress),
                    delta.1 as f32
                        * 0.002
                        * self
                            .settings
                            .look_multiplier(self.weapons.active().ads_progress)
                        * self.settings.look_y_sign(),
                );
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.finish(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    if let Err(e) =
                        r.resize_follow_display_aspect(size, self.settings.render_scale())
                    {
                        self.fail(event_loop, e);
                    }
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.clock = Instant::now();
                if occluded {
                    self.release_capture();
                }
            }
            WindowEvent::Focused(focused) => {
                self.window_focused = focused;
                if focused {
                    // Retry capture after startup and focus changes.
                    self.try_capture();
                } else {
                    self.release_capture();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if !self.captured {
                    if !self.settings_menu && pressed && button == MouseButton::Left {
                        self.capture_requested = true;
                        self.try_capture();
                        self.clock = Instant::now();
                    }
                } else {
                    match button {
                        MouseButton::Left => self.fire_input.set_trigger(pressed),
                        MouseButton::Right => self.fire_input.ads = pressed,
                        _ => {}
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.window_focused = true;
                let pressed = event.state == ElementState::Pressed;
                let gameplay_active = !self.title_screen && !self.settings_menu;
                if self.waiting_for_binding {
                    if pressed && !event.repeat {
                        if let PhysicalKey::Code(code) = event.physical_key {
                            if code == KeyCode::Escape {
                                self.waiting_for_binding = false;
                                self.set_notice("KEY BINDING CANCELLED", "");
                            } else if let Some(key) = code_to_binding_key(code) {
                                let action = BindingAction::ALL
                                    [self.binding_selection % BindingAction::ALL.len()];
                                if self.settings.key_bindings.set(action, key) {
                                    self.waiting_for_binding = false;
                                    self.save_settings();
                                    self.set_notice("KEY BINDING UPDATED", "");
                                } else {
                                    self.set_notice("KEY ALREADY IN USE", "Choose another key");
                                }
                            }
                        }
                    }
                } else {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowUp)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            self.title_selection.move_up();
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            self.title_selection.move_down();
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            match self.title_selection.activate() {
                                TitleMenuAction::FreeTraining => {
                                    self.begin_session(SessionMode::FreeTraining)
                                }
                                TitleMenuAction::TrackingAim => {
                                    self.begin_session(SessionMode::TrackingAim)
                                }
                                TitleMenuAction::Settings => {
                                    self.title_screen = false;
                                    self.settings_menu = true;
                                    self.settings_from_title = true;
                                    self.pause_selection = PauseSelection::default();
                                    self.capture_requested = false;
                                }
                            }
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && (code == KeyCode::Escape
                                    || code
                                        == binding_key_to_code(
                                            self.settings.key_bindings.get(BindingAction::Pause),
                                        )) =>
                        {
                            if self.settings_menu {
                                self.resume_from_pause();
                            } else {
                                self.capture_requested = false;
                                self.release_capture();
                                if !self.title_screen {
                                    self.pause_selection = PauseSelection::default();
                                    self.settings_from_title = false;
                                    self.settings_menu = true;
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            match self.pause_selection.activate() {
                                Some(PauseMenuAction::Resume) => self.resume_from_pause(),
                                Some(PauseMenuAction::ChangeMode) => self.change_mode(),
                                None => {
                                    if self.pause_selection.selected_index() == 5 {
                                        self.waiting_for_binding = true;
                                        self.set_notice("PRESS A KEY TO REBIND", "ESC cancels");
                                    } else if let Some(index) =
                                        self.pause_selection.toggle_setting_index()
                                    {
                                        self.toggle_setting(index);
                                    }
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.pause_selection.move_up();
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.pause_selection.move_down();
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            if self.pause_selection.selected_index() == 5 {
                                self.binding_selection =
                                    (self.binding_selection + BindingAction::ALL.len() - 1)
                                        % BindingAction::ALL.len();
                            } else if let Some(index) = self.pause_selection.setting_index() {
                                self.adjust_runtime_setting(index, -1.0);
                                self.apply_runtime_setting(index);
                                self.save_settings();
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            if self.pause_selection.selected_index() == 5 {
                                self.binding_selection =
                                    (self.binding_selection + 1) % BindingAction::ALL.len();
                            } else if let Some(index) = self.pause_selection.setting_index() {
                                self.adjust_runtime_setting(index, 1.0);
                                self.apply_runtime_setting(index);
                                self.save_settings();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Home)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.settings = GameSettings::default();
                            self.tracking_bot_count = self.settings.bot_count.min(5);
                            self.binding_selection = 0;
                            self.waiting_for_binding = false;
                            for index in [3, 4, 5, 6, 7, 8] {
                                self.apply_runtime_setting(index);
                            }
                            self.apply_fullscreen();
                            self.apply_runtime_setting(9);
                            self.apply_runtime_setting(10);
                            self.next_frame_deadline = None;
                            self.save_settings();
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Left),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.left = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Right),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.right = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Forward),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.forward = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Backward),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.back = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Dash),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.dash = pressed
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && !self.title_screen
                                && !self.settings_menu
                                && code
                                    == binding_key_to_code(
                                        self.settings.key_bindings.get(BindingAction::Jump),
                                    ) =>
                        {
                            self.input.jump = true
                        }
                        _ if !self.captured => {}
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Pistol),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::Pistol);
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::AssaultRifle),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::AssaultRifle);
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Sniper),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::Sniper);
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && !self.infinite_ammo_active()
                                && code
                                    == binding_key_to_code(
                                        self.settings.key_bindings.get(BindingAction::Reload),
                                    ) =>
                        {
                            self.fire_input.reload = true;
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.suspended || self.occluded {
                    return;
                }
                if !self.ensure_renderer() {
                    return;
                }
                if !self.renderer.as_ref().is_some_and(|r| r.drawable()) {
                    return;
                }
                let now = Instant::now();
                let dt = now.duration_since(self.clock).as_secs_f64();
                self.clock = now;
                let gameplay_active = !self.title_screen && !self.settings_menu;
                // Simulation is paused while the pointer is not captured so the
                // "click to play / click to resume" prompt matches a frozen world.
                if Input::should_simulate_gameplay(gameplay_active, self.window_focused)
                    && self.captured
                {
                    if std::mem::take(&mut self.fire_input.reload)
                        && !self.infinite_ammo_active()
                        && !self.weapons.is_switching()
                        && self.weapons.active_mut().reload()
                    {
                        if let Some(audio) = &self.audio {
                            audio.play(Sound::Reload);
                        }
                    }
                    let updates = self.fixed.updates(dt);
                    for _ in 0..updates {
                        let input = self.input;
                        self.input.jump = false;
                        let previous_position = self.player.position;
                        let movement =
                            self.player
                                .move_with_collision(&self.colliders, input, 1. / 60.);
                        if let Some(audio) = &self.audio {
                            if movement.landed {
                                audio.play(Sound::Land);
                            }
                        }
                        let horizontal_distance = glam::Vec2::new(
                            self.player.position.x - previous_position.x,
                            self.player.position.z - previous_position.z,
                        )
                        .length();
                        if movement.landed {
                            self.footstep_clock.reset();
                        } else if self.footstep_clock.advance(
                            1. / 60.,
                            horizontal_distance > 1e-4,
                            self.player.on_ground,
                            input.dash,
                        ) {
                            if let Some(audio) = &self.audio {
                                audio.play(Sound::Footstep);
                            }
                        }
                        self.weapons.advance_with_movement(
                            1. / 60.,
                            self.fire_input.ads,
                            horizontal_distance * 60.0,
                        );
                        let view_right =
                            Vec3::new(self.player.yaw.cos(), 0.0, self.player.yaw.sin());
                        self.combat.advance_with_view(
                            1. / 60.,
                            self.player.position,
                            view_right,
                            &self.colliders,
                        );
                        let (held, pressed) = self.fire_input.take_state();
                        if held || pressed {
                            self.fire(held, pressed);
                        }
                    }
                    // Accept presses between 60 Hz updates; cooldowns advance on the fixed step.
                    if updates == 0 {
                        let (held, pressed) = self.fire_input.take_state();
                        if held || pressed {
                            self.fire(held, pressed);
                        }
                    }
                }
                self.update_title();
                let infinite_ammo_active = self.infinite_ammo_active();
                let notice = self
                    .notice
                    .as_ref()
                    .filter(|(_, started)| started.elapsed() < Duration::from_secs(8))
                    .map(|(message, _)| message.clone());
                if self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, started)| started.elapsed() >= Duration::from_secs(8))
                {
                    self.notice = None;
                }
                let capture_prompt = !self.captured && !self.title_screen && !self.settings_menu;
                let (prompt_title, prompt_subtitle) = if self.window_focused {
                    ("CLICK TO PLAY", "MOUSE LOOK REQUIRES POINTER LOCK")
                } else {
                    ("PAUSED", "CLICK TO RESUME")
                };
                let r = self.renderer.as_mut().unwrap();
                let (w, h) = r.internal_size();
                let mut display_settings = self.settings.clone();
                if !self.settings_from_title && self.session == SessionMode::TrackingAim {
                    display_settings.bot_count = self.tracking_bot_count;
                }
                let matrix = {
                    let aspect = w as f32 / h as f32;
                    let (displayed_weapon, switch_offset) = self.weapons.presentation();
                    crate::ui::hud_mesh_with_menu_and_ammo_and_hit_marker_strength_into(
                        &mut self.hud_mesh,
                        displayed_weapon,
                        &self.combat,
                        aspect,
                        switch_offset,
                        crate::ui::HudOptions {
                            menu: self.settings_menu.then_some((
                                &display_settings,
                                self.pause_selection.selected_index(),
                            )),
                            infinite_ammo: infinite_ammo_active,
                            hit_marker_strength: self.settings.hit_marker_strength,
                        },
                    );
                    if self.title_screen {
                        crate::ui::title_overlay(
                            &mut self.hud_mesh,
                            self.title_selection.selected_index(),
                            aspect,
                        );
                    }
                    if capture_prompt {
                        crate::ui::capture_prompt_overlay(&mut self.hud_mesh, aspect);
                    }
                    crate::ui::world_mesh_into(&mut self.world_mesh, &self.combat);
                    r.update_extras(&self.world_mesh, &self.hud_mesh);
                    if self.title_screen {
                        crate::ui::title_text_into(
                            &mut self.text_mesh,
                            self.title_selection.selected_index(),
                            [w, h],
                        );
                    } else if self.settings_menu {
                        crate::ui::settings_text_with_bindings_into(
                            &mut self.text_mesh,
                            &display_settings,
                            self.pause_selection.selected_index(),
                            [w, h],
                            self.settings_from_title,
                            self.binding_selection,
                            self.waiting_for_binding,
                        )
                    } else {
                        crate::ui::game_hud_text_into(
                            &mut self.text_mesh,
                            crate::ui::GameHudInfo {
                                mode: self.session,
                                weapon: self.weapons.active_kind(),
                                ammo: self.weapons.active().ammo,
                                magazine: self.weapons.active().data.magazine,
                                kills: self.combat.kills,
                                infinite_ammo: infinite_ammo_active,
                                key_bindings: self.settings.key_bindings,
                            },
                            [w, h],
                        );
                    }
                    if capture_prompt {
                        crate::ui::capture_prompt_text(
                            &mut self.text_mesh,
                            prompt_title,
                            prompt_subtitle,
                            [w, h],
                        );
                    }
                    if let Some(notice) = &notice {
                        let notice_y = if self.title_screen {
                            0.34
                        } else if self.settings_menu {
                            0.575
                        } else {
                            0.76
                        };
                        crate::ui::append_notice_text(
                            &mut self.text_mesh,
                            notice,
                            [w, h],
                            notice_y,
                        );
                    }
                    r.update_text(&self.text_mesh);
                    self.player
                        .view_matrix_with(aspect, self.weapons.fov_with_base(self.settings.fov))
                };
                match r.render(matrix) {
                    Ok(true) => {}
                    Ok(false) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Err(e) = r.resize(r.window.inner_size()) {
                            self.fail(event_loop, e);
                        }
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        console_error("Surface timeout; retrying");
                    }
                    Err(e) => self.fail(event_loop, format!("描画失敗: {e}")),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let drawable = self.renderer.as_ref().is_some_and(|r| r.drawable())
            && !self.suspended
            && !self.occluded;
        if drawable {
            if let Some(fps) = self.settings.fps_limit() {
                let now = Instant::now();
                let deadline = match self.next_frame_deadline {
                    Some(deadline) if deadline > now => deadline,
                    _ => {
                        let deadline = now + Duration::from_secs_f64(1.0 / f64::from(fps));
                        self.next_frame_deadline = Some(deadline);
                        if let Some(renderer) = &self.renderer {
                            renderer.window.request_redraw();
                        }
                        deadline
                    }
                };
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            } else {
                self.next_frame_deadline = None;
                event_loop.set_control_flow(ControlFlow::Poll);
                if let Some(renderer) = &self.renderer {
                    renderer.window.request_redraw();
                }
            }
        } else if self.pending.is_some() {
            // Keep polling until the asynchronous renderer initialization lands.
            event_loop.set_control_flow(ControlFlow::Poll);
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

/// Builds the app and runs the browser event loop.
pub fn run() {
    if !webgpu_available() {
        show_fallback();
        return;
    }
    let settings = GameSettings::load();
    let tracking_bot_count = settings.bot_count.min(5);
    let arena_scale = world::arena_scale_for_bot_count(settings.bot_count);
    let app = App {
        renderer: None,
        pending: None,
        world_mesh: Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        },
        hud_mesh: Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        },
        text_mesh: TextMesh::default(),
        clock: Instant::now(),
        player: Player::new(),
        colliders: world::arena_colliders_scaled(arena_scale),
        input: Input::default(),
        footstep_clock: FootstepClock::default(),
        weapons: Loadout::default(),
        shot_spread_rng: ShotSpreadRng::from_entropy(),
        fire_input: FireInput::default(),
        combat: Combat::default(),
        settings,
        tracking_bot_count,
        session: SessionMode::FreeTraining,
        title_screen: true,
        settings_menu: false,
        settings_from_title: false,
        title_selection: TitleSelection::default(),
        pause_selection: PauseSelection::default(),
        binding_selection: 0,
        waiting_for_binding: false,
        audio: None,
        title: String::new(),
        capture_requested: false,
        captured: false,
        window_focused: false,
        fixed: FixedStep::new(60, 5),
        next_frame_deadline: None,
        occluded: false,
        suspended: false,
        error: None,
        notice: None,
    };
    let event_loop = EventLoop::builder().build().expect("event loop");
    event_loop.spawn_app(app);
}
