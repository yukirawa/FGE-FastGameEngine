use glam::Vec2;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// Compact, allocation-friendly input snapshot updated from `winit::event::WindowEvent`.
/// Uses flat contiguous `Vec` buffers (typically < 8 active elements) for cache locality.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    keys_down: Vec<KeyCode>,
    keys_just_pressed: Vec<KeyCode>,
    keys_just_released: Vec<KeyCode>,
    mouse_down: Vec<MouseButton>,
    cursor_position: Vec2,
    scroll_delta: Vec2,
    close_requested: bool,
    resized: Option<(u32, u32)>,
}

impl InputState {
    /// Creates an empty `InputState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears single-frame edge transitions (`just_pressed`, `just_released`, `scroll_delta`, `resized`).
    pub fn begin_frame(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.scroll_delta = Vec2::ZERO;
        self.resized = None;
    }

    /// Processes a `winit::event::WindowEvent` and updates internal key/mouse/window state.
    pub fn process_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::Resized(size) => {
                self.resized = Some((size.width, size.height));
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.scroll_delta += Vec2::new(*x, *y);
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    self.scroll_delta += Vec2::new(pos.x as f32, pos.y as f32);
                }
            },
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if !self.mouse_down.contains(button) {
                        self.mouse_down.push(*button);
                    }
                }
                ElementState::Released => {
                    if let Some(idx) = self.mouse_down.iter().position(|b| b == button) {
                        self.mouse_down.swap_remove(idx);
                    }
                }
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !self.keys_down.contains(&code) {
                                self.keys_down.push(code);
                                if !self.keys_just_pressed.contains(&code) {
                                    self.keys_just_pressed.push(code);
                                }
                            }
                        }
                        ElementState::Released => {
                            if let Some(idx) = self.keys_down.iter().position(|k| *k == code) {
                                self.keys_down.swap_remove(idx);
                            }
                            if !self.keys_just_released.contains(&code) {
                                self.keys_just_released.push(code);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn cursor_position(&self) -> Vec2 {
        self.cursor_position
    }

    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    pub fn take_resized(&mut self) -> Option<(u32, u32)> {
        self.resized.take()
    }
}
