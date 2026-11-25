use fge_core::input_core::{CoreInputState, FgeKey, FgeMouseButton};
use winit::{
    event::{MouseButton, WindowEvent},
    keyboard::KeyCode,
};

/// Maps a `winit::keyboard::KeyCode` to an engine-agnostic `FgeKey`.
pub fn map_winit_keycode(_code: KeyCode) -> FgeKey {
    todo!("map_winit_keycode")
}

/// Maps a `winit::event::MouseButton` to `Option<FgeMouseButton>`.
pub fn map_winit_mouse_button(_button: MouseButton) -> Option<FgeMouseButton> {
    todo!("map_winit_mouse_button")
}

/// Feeds a `winit::event::WindowEvent` directly into `CoreInputState`.
pub fn apply_window_event_to_core_input(_core_input: &mut CoreInputState, _event: &WindowEvent) {
    todo!("apply_window_event_to_core_input")
}
