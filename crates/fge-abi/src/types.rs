use fge_audio::engine::SoundId;
use fge_core::{emitter::EmitterId, handle::EntityId, spatial::CollisionPair};

pub use fge_core::ui::FgeTextBuf;

/// C-ABI 64-bit generational handle (`typedef struct FgeEntity` in `fge.h`).
pub type FgeEntity = EntityId;

/// C-ABI 64-bit generational sound handle (`typedef struct FgeSound` in `fge.h`).
pub type FgeSound = SoundId;

/// C-ABI 64-bit generational emitter handle (`typedef struct FgeEmitter` in `fge.h`).
pub type FgeEmitter = EmitterId;

/// C-ABI 64-bit generational texture handle (`typedef struct FgeTexture` in `fge.h`).
pub type FgeTexture = EntityId;

/// C-ABI collision pair (`typedef struct FgeCollisionPair` in `fge.h`).
pub type FgeCollisionPair = CollisionPair;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FgeVec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FgeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FgeUvRect {
    pub u0: u16,
    pub v0: u16,
    pub u1: u16,
    pub v1: u16,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FgeCamera {
    pub center_x: f32,
    pub center_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub zoom: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FgeAiEvent {
    pub kind: u32,
    pub target: FgeEntity,
    pub arg0: f32,
    pub arg1: f32,
}

const _: () = assert!(core::mem::size_of::<FgeEntity>() == 8);
const _: () = assert!(core::mem::size_of::<FgeSound>() == 8);
const _: () = assert!(core::mem::size_of::<FgeEmitter>() == 8);
const _: () = assert!(core::mem::size_of::<FgeTexture>() == 8);
const _: () = assert!(core::mem::size_of::<FgeTextBuf>() == 124);
const _: () = assert!(core::mem::size_of::<FgeVec2>() == 8);
const _: () = assert!(core::mem::size_of::<FgeColor>() == 4);
const _: () = assert!(core::mem::size_of::<FgeUvRect>() == 8);
const _: () = assert!(core::mem::size_of::<FgeCamera>() == 20);
const _: () = assert!(core::mem::size_of::<FgeCollisionPair>() == 16);
const _: () = assert!(core::mem::size_of::<FgeAiEvent>() == 20);
