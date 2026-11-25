use glam::Vec2;
use winit::{
    event::{MouseButton, WindowEvent},
    keyboard::KeyCode,
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
        todo!("InputState::begin_frame")
    }

    /// Processes a `winit::event::WindowEvent` and updates internal key/mouse/window state.
    pub fn process_window_event(&mut self, _event: &WindowEvent) {
        todo!("InputState::process_window_event")
    }

    pub fn is_key_down(&self, _key: KeyCode) -> bool {
        todo!("InputState::is_key_down")
    }

    pub fn is_key_just_pressed(&self, _key: KeyCode) -> bool {
        todo!("InputState::is_key_just_pressed")
    }

    pub fn is_key_just_released(&self, _key: KeyCode) -> bool {
        todo!("InputState::is_key_just_released")
    }

    pub fn is_mouse_down(&self, _button: MouseButton) -> bool {
        todo!("InputState::is_mouse_down")
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
