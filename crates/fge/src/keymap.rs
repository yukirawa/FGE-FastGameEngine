use fge_core::input_core::{CoreInputState, FgeKey, FgeMouseButton};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// Maps a `winit::keyboard::KeyCode` to an engine-agnostic `FgeKey`.
pub fn map_winit_keycode(code: KeyCode) -> FgeKey {
    match code {
        KeyCode::ArrowUp => FgeKey::Up,
        KeyCode::ArrowDown => FgeKey::Down,
        KeyCode::ArrowLeft => FgeKey::Left,
        KeyCode::ArrowRight => FgeKey::Right,
        KeyCode::KeyW => FgeKey::W,
        KeyCode::KeyA => FgeKey::A,
        KeyCode::KeyS => FgeKey::S,
        KeyCode::KeyD => FgeKey::D,
        KeyCode::KeyZ => FgeKey::Z,
        KeyCode::KeyX => FgeKey::X,
        KeyCode::KeyC => FgeKey::C,
        KeyCode::Space => FgeKey::Space,
        KeyCode::ShiftLeft => FgeKey::LShift,
        KeyCode::ShiftRight => FgeKey::RShift,
        KeyCode::Enter => FgeKey::Enter,
        KeyCode::Escape => FgeKey::Escape,
        KeyCode::Digit0 => FgeKey::Digit0,
        KeyCode::Digit1 => FgeKey::Digit1,
        KeyCode::Digit2 => FgeKey::Digit2,
        KeyCode::Digit3 => FgeKey::Digit3,
        KeyCode::Digit4 => FgeKey::Digit4,
        KeyCode::Digit5 => FgeKey::Digit5,
        KeyCode::Digit6 => FgeKey::Digit6,
        KeyCode::Digit7 => FgeKey::Digit7,
        KeyCode::Digit8 => FgeKey::Digit8,
        KeyCode::Digit9 => FgeKey::Digit9,
        KeyCode::KeyQ => FgeKey::Q,
        KeyCode::KeyE => FgeKey::E,
        KeyCode::KeyR => FgeKey::R,
        KeyCode::KeyF => FgeKey::F,
        KeyCode::Tab => FgeKey::Tab,
        _ => FgeKey::Unknown,
    }
}

/// Maps a `winit::event::MouseButton` to `Option<FgeMouseButton>`.
pub fn map_winit_mouse_button(button: MouseButton) -> Option<FgeMouseButton> {
    match button {
        MouseButton::Left => Some(FgeMouseButton::Left),
        MouseButton::Right => Some(FgeMouseButton::Right),
        MouseButton::Middle => Some(FgeMouseButton::Middle),
        _ => None,
    }
}

/// Feeds a `winit::event::WindowEvent` directly into `CoreInputState`.
pub fn apply_window_event_to_core_input(core_input: &mut CoreInputState, event: &WindowEvent) {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            if let PhysicalKey::Code(code) = event.physical_key {
                let key = map_winit_keycode(code);
                core_input.set_key_state(key, event.state == ElementState::Pressed);
            }
        }
        WindowEvent::MouseInput { state, button, .. } => {
            if let Some(btn) = map_winit_mouse_button(*button) {
                core_input.set_mouse_button(btn, *state == ElementState::Pressed);
            }
        }
        WindowEvent::CursorMoved { position, .. } => {
            core_input.set_mouse_pos(position.x as f32, position.y as f32);
        }
        WindowEvent::MouseWheel { delta, .. } => match delta {
            MouseScrollDelta::LineDelta(x, y) => core_input.add_mouse_wheel(*x, *y),
            MouseScrollDelta::PixelDelta(pos) => {
                core_input.add_mouse_wheel(pos.x as f32, pos.y as f32);
            }
        },
        _ => {}
    }
}
