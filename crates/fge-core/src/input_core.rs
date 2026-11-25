/// Engine-agnostic key code (`#[repr(u32)]` matching `FgeKey` in `include/fge.h`).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FgeKey {
    Unknown = 0,
    Up = 1,
    Down = 2,
    Left = 3,
    Right = 4,
    W = 5,
    A = 6,
    S = 7,
    D = 8,
    Z = 9,
    X = 10,
    C = 11,
    Space = 12,
    LShift = 13,
    RShift = 14,
    Enter = 15,
    Escape = 16,
    Digit0 = 17,
    Digit1 = 18,
    Digit2 = 19,
    Digit3 = 20,
    Digit4 = 21,
    Digit5 = 22,
    Digit6 = 23,
    Digit7 = 24,
    Digit8 = 25,
    Digit9 = 26,
    Q = 27,
    E = 28,
    R = 29,
    F = 30,
    Tab = 31,
}

impl FgeKey {
    pub const COUNT: usize = 32;

    pub fn from_u32(_code: u32) -> Option<Self> {
        todo!("FgeKey::from_u32")
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FgeMouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

impl FgeMouseButton {
    pub fn from_u32(_code: u32) -> Option<Self> {
        todo!("FgeMouseButton::from_u32")
    }
}

/// Zero-allocation bitset input state shared by `winit`, `fge-abi`, and headless/AI simulation.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct CoreInputState {
    pub down_bits: u64,
    pub pressed_bits: u64,
    pub released_bits: u64,
    pub mouse_down_bits: u8,
    pub mouse_pos: [f32; 2],
    pub mouse_wheel: [f32; 2],
}

impl CoreInputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears single-frame edge bits (`pressed_bits`, `released_bits`, `mouse_wheel`).
    pub fn begin_frame(&mut self) {
        todo!("CoreInputState::begin_frame")
    }

    pub fn set_key_state(&mut self, _key: FgeKey, _pressed: bool) {
        todo!("CoreInputState::set_key_state")
    }

    pub fn set_mouse_button(&mut self, _button: FgeMouseButton, _pressed: bool) {
        todo!("CoreInputState::set_mouse_button")
    }

    pub fn set_mouse_pos(&mut self, _x: f32, _y: f32) {
        self.mouse_pos = [_x, _y];
    }

    pub fn add_mouse_wheel(&mut self, _dx: f32, _dy: f32) {
        self.mouse_wheel[0] += _dx;
        self.mouse_wheel[1] += _dy;
    }

    pub fn is_key_down(&self, _key: FgeKey) -> bool {
        todo!("CoreInputState::is_key_down")
    }

    pub fn is_key_pressed(&self, _key: FgeKey) -> bool {
        todo!("CoreInputState::is_key_pressed")
    }

    pub fn is_key_released(&self, _key: FgeKey) -> bool {
        todo!("CoreInputState::is_key_released")
    }

    pub fn is_mouse_down(&self, _button: FgeMouseButton) -> bool {
        todo!("CoreInputState::is_mouse_down")
    }
}
