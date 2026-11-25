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

    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Unknown),
            1 => Some(Self::Up),
            2 => Some(Self::Down),
            3 => Some(Self::Left),
            4 => Some(Self::Right),
            5 => Some(Self::W),
            6 => Some(Self::A),
            7 => Some(Self::S),
            8 => Some(Self::D),
            9 => Some(Self::Z),
            10 => Some(Self::X),
            11 => Some(Self::C),
            12 => Some(Self::Space),
            13 => Some(Self::LShift),
            14 => Some(Self::RShift),
            15 => Some(Self::Enter),
            16 => Some(Self::Escape),
            17 => Some(Self::Digit0),
            18 => Some(Self::Digit1),
            19 => Some(Self::Digit2),
            20 => Some(Self::Digit3),
            21 => Some(Self::Digit4),
            22 => Some(Self::Digit5),
            23 => Some(Self::Digit6),
            24 => Some(Self::Digit7),
            25 => Some(Self::Digit8),
            26 => Some(Self::Digit9),
            27 => Some(Self::Q),
            28 => Some(Self::E),
            29 => Some(Self::R),
            30 => Some(Self::F),
            31 => Some(Self::Tab),
            _ => None,
        }
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
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Left),
            1 => Some(Self::Right),
            2 => Some(Self::Middle),
            _ => None,
        }
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
    #[inline]
    pub fn begin_frame(&mut self) {
        self.pressed_bits = 0;
        self.released_bits = 0;
        self.mouse_wheel = [0.0, 0.0];
    }

    #[inline]
    pub fn set_key_state(&mut self, key: FgeKey, pressed: bool) {
        if key == FgeKey::Unknown {
            return;
        }
        let bit = 1u64 << (key as u32);
        if pressed {
            if (self.down_bits & bit) == 0 {
                self.pressed_bits |= bit;
            }
            self.down_bits |= bit;
        } else {
            if (self.down_bits & bit) != 0 {
                self.released_bits |= bit;
            }
            self.down_bits &= !bit;
        }
    }

    #[inline]
    pub fn set_mouse_button(&mut self, button: FgeMouseButton, pressed: bool) {
        let bit = 1u8 << (button as u32);
        if pressed {
            self.mouse_down_bits |= bit;
        } else {
            self.mouse_down_bits &= !bit;
        }
    }

    #[inline]
    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        self.mouse_pos = [x, y];
    }

    #[inline]
    pub fn add_mouse_wheel(&mut self, dx: f32, dy: f32) {
        self.mouse_wheel[0] += dx;
        self.mouse_wheel[1] += dy;
    }

    #[inline]
    pub fn is_key_down(&self, key: FgeKey) -> bool {
        key != FgeKey::Unknown && (self.down_bits & (1u64 << (key as u32))) != 0
    }

    #[inline]
    pub fn is_key_pressed(&self, key: FgeKey) -> bool {
        key != FgeKey::Unknown && (self.pressed_bits & (1u64 << (key as u32))) != 0
    }

    #[inline]
    pub fn is_key_released(&self, key: FgeKey) -> bool {
        key != FgeKey::Unknown && (self.released_bits & (1u64 << (key as u32))) != 0
    }

    #[inline]
    pub fn is_mouse_down(&self, button: FgeMouseButton) -> bool {
        (self.mouse_down_bits & (1u8 << (button as u32))) != 0
    }
}
