use std::{borrow::Cow, fmt::Write as _, ops::Range};

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
    /// Copies up to 120 UTF-8 bytes from `s`, truncating cleanly at a valid UTF-8 character boundary.
    pub fn from_str_lossy(s: &str) -> Self {
        let mut buf = Self::default();
        let mut end = s.len().min(buf.bytes.len());
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        buf.bytes[..end].copy_from_slice(&s.as_bytes()[..end]);
        buf.len = end as u32;
        buf
    }

    /// Returns a safe UTF-8 view of the buffer (zero-allocation `Cow::Borrowed` for valid UTF-8).
    #[inline]
    pub fn as_str(&self) -> Cow<'_, str> {
        let len = (self.len as usize).min(self.bytes.len());
        String::from_utf8_lossy(&self.bytes[..len])
    }
}

/// Truncates `s` cleanly at a UTF-8 character boundary to at most `max_chars` Unicode scalar values.
#[inline]
pub fn truncate_utf8_chars(s: &str, max_chars: u32) -> &str {
    if max_chars == 0 {
        return "";
    }
    match s.char_indices().nth(max_chars as usize) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
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

    pub fn with_capacity(cmd_capacity: usize, text_bytes: usize) -> Self {
        Self {
            text_arena: String::with_capacity(text_bytes),
            cmds: Vec::with_capacity(cmd_capacity),
        }
    }

    /// Clears queued commands and text spans while preserving allocated heap capacity for zero-alloc reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.text_arena.clear();
        self.cmds.clear();
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

    #[inline]
    pub fn resolve_span(&self, span: Range<u32>) -> &str {
        let start = span.start as usize;
        let end = span.end as usize;
        if start <= end && end <= self.text_arena.len() {
            self.text_arena.get(start..end).unwrap_or("")
        } else {
            ""
        }
    }

    #[inline]
    fn intern_text(&mut self, text: &str) -> Range<u32> {
        let start = self.text_arena.len();
        self.text_arena.push_str(text);
        let end = self.text_arena.len();
        debug_assert!(end <= u32::MAX as usize);
        (start as u32)..(end as u32)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_panel(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        bg_color: [u8; 4],
        border_color: [u8; 4],
        border_px: f32,
    ) {
        self.cmds.push(UiDrawCmd::Panel {
            x,
            y,
            w,
            h,
            bg_color,
            border_color,
            border_px,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_bar(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        ratio: f32,
        fill_color: [u8; 4],
        bg_color: [u8; 4],
    ) {
        let clamped_ratio = if ratio.is_finite() {
            ratio.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.cmds.push(UiDrawCmd::Bar {
            x,
            y,
            w,
            h,
            ratio: clamped_ratio,
            fill_color,
            bg_color,
        });
    }

    pub fn push_screen_text(
        &mut self,
        x: f32,
        y: f32,
        font_size: f32,
        color: [u8; 4],
        text: &str,
    ) {
        let text_span = self.intern_text(text);
        self.cmds.push(UiDrawCmd::ScreenText {
            x,
            y,
            font_size,
            color,
            text_span,
        });
    }

    pub fn push_number(
        &mut self,
        x: f32,
        y: f32,
        font_size: f32,
        value: i64,
        color: [u8; 4],
    ) {
        let start = self.text_arena.len();
        let _ = write!(&mut self.text_arena, "{value}");
        let end = self.text_arena.len();
        debug_assert!(end <= u32::MAX as usize);
        self.cmds.push(UiDrawCmd::ScreenText {
            x,
            y,
            font_size,
            color,
            text_span: (start as u32)..(end as u32),
        });
    }

    pub fn push_world_text(
        &mut self,
        world_x: f32,
        world_y: f32,
        font_size: f32,
        color: [u8; 4],
        text: &str,
    ) {
        let text_span = self.intern_text(text);
        self.cmds.push(UiDrawCmd::WorldText {
            world_x,
            world_y,
            font_size,
            color,
            text_span,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_dialog_box(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        speaker: &str,
        message: &str,
        visible_chars: u32,
    ) {
        let speaker_span = self.intern_text(speaker);
        let message_span = self.intern_text(message);
        self.cmds.push(UiDrawCmd::DialogBox {
            x,
            y,
            w,
            h,
            speaker_span,
            message_span,
            visible_chars,
        });
    }
}
