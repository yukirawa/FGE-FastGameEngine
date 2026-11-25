use std::{borrow::Cow, ops::Range};

/// Fixed-capacity by-value UTF-8 buffer (`#[repr(C)]`, 124 bytes) matching `FgeTextBuf` in `fge.h`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FgeTextBuf {
    pub bytes: [u8; 120],
    pub len: u32,
}

const _: () = assert!(core::mem::size_of::<FgeTextBuf>() == 124);

impl Default for FgeTextBuf {
    fn default() -> Self {
        Self {
            bytes: [0u8; 120],
            len: 0,
        }
    }
}

impl FgeTextBuf {
    pub fn from_str_lossy(_s: &str) -> Self {
        todo!("implemented by impl-core5")
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        todo!("implemented by impl-core5")
    }
}

/// Truncates `s` cleanly at a UTF-8 character boundary to at most `max_chars` Unicode scalar values.
pub fn truncate_utf8_chars(_s: &str, _max_chars: u32) -> &str {
    todo!("implemented by impl-core5")
}

/// Immediate-mode UI / text draw command referencing byte spans inside [`UiCommandQueue::text_arena`].
#[derive(Clone, Debug, PartialEq)]
pub enum UiDrawCmd {
    Panel {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        bg_color: [u8; 4],
        border_color: [u8; 4],
        border_px: f32,
    },
    Bar {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        ratio: f32,
        fill_color: [u8; 4],
        bg_color: [u8; 4],
    },
    ScreenText {
        x: f32,
        y: f32,
        font_size: f32,
        color: [u8; 4],
        text_span: Range<u32>,
    },
    WorldText {
        world_x: f32,
        world_y: f32,
        font_size: f32,
        color: [u8; 4],
        text_span: Range<u32>,
    },
    DialogBox {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        speaker_span: Range<u32>,
        message_span: Range<u32>,
        visible_chars: u32,
    },
}

/// Zero-per-frame-allocation immediate-mode UI queue backed by a single reusable string arena.
#[derive(Clone, Debug, Default)]
pub struct UiCommandQueue {
    pub(crate) text_arena: String,
    pub(crate) cmds: Vec<UiDrawCmd>,
}

impl UiCommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        todo!("implemented by impl-core5")
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    #[inline]
    pub fn commands(&self) -> &[UiDrawCmd] {
        &self.cmds
    }

    #[inline]
    pub fn text_arena(&self) -> &str {
        &self.text_arena
    }

    pub fn resolve_span(&self, _span: Range<u32>) -> &str {
        todo!("implemented by impl-core5")
    }

    pub fn push_panel(
        &mut self,
        _x: f32,
        _y: f32,
        _w: f32,
        _h: f32,
        _bg_color: [u8; 4],
        _border_color: [u8; 4],
        _border_px: f32,
    ) {
        todo!("implemented by impl-core5")
    }

    pub fn push_bar(
        &mut self,
        _x: f32,
        _y: f32,
        _w: f32,
        _h: f32,
        _ratio: f32,
        _fill_color: [u8; 4],
        _bg_color: [u8; 4],
    ) {
        todo!("implemented by impl-core5")
    }

    pub fn push_screen_text(&mut self, _x: f32, _y: f32, _font_size: f32, _color: [u8; 4], _text: &str) {
        todo!("implemented by impl-core5")
    }

    pub fn push_number(&mut self, _x: f32, _y: f32, _font_size: f32, _value: i64, _color: [u8; 4]) {
        todo!("implemented by impl-core5")
    }

    pub fn push_world_text(
        &mut self,
        _world_x: f32,
        _world_y: f32,
        _font_size: f32,
        _color: [u8; 4],
        _text: &str,
    ) {
        todo!("implemented by impl-core5")
    }

    pub fn push_dialog_box(
        &mut self,
        _x: f32,
        _y: f32,
        _w: f32,
        _h: f32,
        _speaker: &str,
        _message: &str,
        _visible_chars: u32,
    ) {
        todo!("implemented by impl-core5")
    }
}
