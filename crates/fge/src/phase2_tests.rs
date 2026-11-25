// Phase 2 unit tests for `fge::keymap`.

use fge_core::input_core::{CoreInputState, FgeKey, FgeMouseButton};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::KeyCode,
};

use crate::keymap::{apply_window_event_to_core_input, map_winit_keycode, map_winit_mouse_button};

#[test]
fn map_winit_keycode_maps_gameplay_keys_and_digits() {
    // Arrow keys
    assert_eq!(map_winit_keycode(KeyCode::ArrowUp), FgeKey::Up);
    assert_eq!(map_winit_keycode(KeyCode::ArrowDown), FgeKey::Down);
    assert_eq!(map_winit_keycode(KeyCode::ArrowLeft), FgeKey::Left);
    assert_eq!(map_winit_keycode(KeyCode::ArrowRight), FgeKey::Right);

    // WASD
    assert_eq!(map_winit_keycode(KeyCode::KeyW), FgeKey::W);
    assert_eq!(map_winit_keycode(KeyCode::KeyA), FgeKey::A);
    assert_eq!(map_winit_keycode(KeyCode::KeyS), FgeKey::S);
    assert_eq!(map_winit_keycode(KeyCode::KeyD), FgeKey::D);

    // Action & utility keys: Z/X/C, Space, Shift, Escape, Enter, Q/E/R/F, Tab
    assert_eq!(map_winit_keycode(KeyCode::KeyZ), FgeKey::Z);
    assert_eq!(map_winit_keycode(KeyCode::KeyX), FgeKey::X);
    assert_eq!(map_winit_keycode(KeyCode::KeyC), FgeKey::C);
    assert_eq!(map_winit_keycode(KeyCode::Space), FgeKey::Space);
    assert_eq!(map_winit_keycode(KeyCode::ShiftLeft), FgeKey::LShift);
    assert_eq!(map_winit_keycode(KeyCode::ShiftRight), FgeKey::RShift);
    assert_eq!(map_winit_keycode(KeyCode::Escape), FgeKey::Escape);
    assert_eq!(map_winit_keycode(KeyCode::Enter), FgeKey::Enter);
    assert_eq!(map_winit_keycode(KeyCode::KeyQ), FgeKey::Q);
    assert_eq!(map_winit_keycode(KeyCode::KeyE), FgeKey::E);
    assert_eq!(map_winit_keycode(KeyCode::KeyR), FgeKey::R);
    assert_eq!(map_winit_keycode(KeyCode::KeyF), FgeKey::F);
    assert_eq!(map_winit_keycode(KeyCode::Tab), FgeKey::Tab);

    // Digits 0..=9
    assert_eq!(map_winit_keycode(KeyCode::Digit0), FgeKey::Digit0);
    assert_eq!(map_winit_keycode(KeyCode::Digit1), FgeKey::Digit1);
    assert_eq!(map_winit_keycode(KeyCode::Digit2), FgeKey::Digit2);
    assert_eq!(map_winit_keycode(KeyCode::Digit3), FgeKey::Digit3);
    assert_eq!(map_winit_keycode(KeyCode::Digit4), FgeKey::Digit4);
    assert_eq!(map_winit_keycode(KeyCode::Digit5), FgeKey::Digit5);
    assert_eq!(map_winit_keycode(KeyCode::Digit6), FgeKey::Digit6);
    assert_eq!(map_winit_keycode(KeyCode::Digit7), FgeKey::Digit7);
    assert_eq!(map_winit_keycode(KeyCode::Digit8), FgeKey::Digit8);
    assert_eq!(map_winit_keycode(KeyCode::Digit9), FgeKey::Digit9);

    // Unmapped keys resolve to FgeKey::Unknown
    assert_eq!(map_winit_keycode(KeyCode::F12), FgeKey::Unknown);
    assert_eq!(map_winit_keycode(KeyCode::ControlLeft), FgeKey::Unknown);
}

#[test]
fn map_winit_mouse_button_maps_primary_buttons() {
    assert_eq!(
        map_winit_mouse_button(MouseButton::Left),
        Some(FgeMouseButton::Left)
    );
    assert_eq!(
        map_winit_mouse_button(MouseButton::Right),
        Some(FgeMouseButton::Right)
    );
    assert_eq!(
        map_winit_mouse_button(MouseButton::Middle),
        Some(FgeMouseButton::Middle)
    );
    assert_eq!(map_winit_mouse_button(MouseButton::Back), None);
    assert_eq!(map_winit_mouse_button(MouseButton::Forward), None);
    assert_eq!(map_winit_mouse_button(MouseButton::Other(4)), None);
}

#[test]
fn apply_window_event_cursor_moved_updates_mouse_pos() {
    let mut core = CoreInputState::new();
    let device_id = DeviceId::dummy();

    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::CursorMoved {
            device_id,
            position: PhysicalPosition::new(412.5, 198.25),
        },
    );
    assert_eq!(core.mouse_pos, [412.5, 198.25]);

    core.begin_frame();
    assert_eq!(core.mouse_pos, [412.5, 198.25]);
}

#[test]
fn apply_window_event_mouse_wheel_accumulates_and_resets_on_begin_frame() {
    let mut core = CoreInputState::new();
    let device_id = DeviceId::dummy();

    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::LineDelta(1.5, -2.0),
            phase: TouchPhase::Moved,
        },
    );
    assert_eq!(core.mouse_wheel, [1.5, -2.0]);

    core.begin_frame();
    assert_eq!(core.mouse_wheel, [0.0, 0.0]);

    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseWheel {
            device_id,
            delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(16.0, -32.0)),
            phase: TouchPhase::Moved,
        },
    );
    assert!(core.mouse_wheel[0] > 0.0);
    assert!(core.mouse_wheel[1] < 0.0);

    core.begin_frame();
    assert_eq!(core.mouse_wheel, [0.0, 0.0]);
}

#[test]
fn apply_window_event_mouse_buttons_press_and_release() {
    let mut core = CoreInputState::new();
    let device_id = DeviceId::dummy();

    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseInput {
            device_id,
            state: ElementState::Pressed,
            button: MouseButton::Left,
        },
    );
    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseInput {
            device_id,
            state: ElementState::Pressed,
            button: MouseButton::Middle,
        },
    );
    // Unmapped mouse button is safely ignored.
    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseInput {
            device_id,
            state: ElementState::Pressed,
            button: MouseButton::Back,
        },
    );

    assert!(core.is_mouse_down(FgeMouseButton::Left));
    assert!(!core.is_mouse_down(FgeMouseButton::Right));
    assert!(core.is_mouse_down(FgeMouseButton::Middle));

    core.begin_frame();
    assert!(core.is_mouse_down(FgeMouseButton::Left));
    assert!(core.is_mouse_down(FgeMouseButton::Middle));

    apply_window_event_to_core_input(
        &mut core,
        &WindowEvent::MouseInput {
            device_id,
            state: ElementState::Released,
            button: MouseButton::Left,
        },
    );
    assert!(!core.is_mouse_down(FgeMouseButton::Left));
    assert!(core.is_mouse_down(FgeMouseButton::Middle));
}

#[test]
fn core_input_mapped_winit_keys_track_down_pressed_and_released() {
    let mut core = CoreInputState::new();
    let tested_codes = [
        (KeyCode::ArrowUp, FgeKey::Up),
        (KeyCode::ArrowDown, FgeKey::Down),
        (KeyCode::ArrowLeft, FgeKey::Left),
        (KeyCode::ArrowRight, FgeKey::Right),
        (KeyCode::KeyW, FgeKey::W),
        (KeyCode::KeyA, FgeKey::A),
        (KeyCode::KeyS, FgeKey::S),
        (KeyCode::KeyD, FgeKey::D),
        (KeyCode::KeyZ, FgeKey::Z),
        (KeyCode::KeyX, FgeKey::X),
        (KeyCode::KeyC, FgeKey::C),
        (KeyCode::Space, FgeKey::Space),
        (KeyCode::ShiftLeft, FgeKey::LShift),
        (KeyCode::ShiftRight, FgeKey::RShift),
        (KeyCode::Escape, FgeKey::Escape),
        (KeyCode::Digit0, FgeKey::Digit0),
        (KeyCode::Digit5, FgeKey::Digit5),
        (KeyCode::Digit9, FgeKey::Digit9),
    ];

    for (code, expected_key) in tested_codes {
        let mapped = map_winit_keycode(code);
        assert_eq!(mapped, expected_key);
        core.set_key_state(mapped, true);
        assert!(core.is_key_down(mapped));
        assert!(core.is_key_pressed(mapped));
        assert!(!core.is_key_released(mapped));
    }

    core.begin_frame();
    for (_, expected_key) in tested_codes {
        assert!(core.is_key_down(expected_key));
        assert!(!core.is_key_pressed(expected_key));
        assert!(!core.is_key_released(expected_key));
    }

    for (_, expected_key) in tested_codes {
        core.set_key_state(expected_key, false);
        assert!(!core.is_key_down(expected_key));
        assert!(core.is_key_released(expected_key));
    }

    // Unrelated window events should leave state intact without panicking.
    apply_window_event_to_core_input(&mut core, &WindowEvent::Focused(true));
    assert!(!core.is_key_down(FgeKey::Space));
}
