use glam::{Mat4, Vec2};

use crate::error::CoreError;

/// Axis-aligned 2D bounding box in world coordinates (`+X` right, `+Y` down).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2D {
    /// Creates an `Aabb2D` from explicit `min` and `max` corners.
    pub fn new(_min: Vec2, _max: Vec2) -> Self {
        todo!("Camera2D Aabb2D::new")
    }

    /// Creates an `Aabb2D` from a center point and non-negative half-extents.
    pub fn from_center_half_extents(_center: Vec2, _half_extents: Vec2) -> Self {
        todo!("Camera2D Aabb2D::from_center_half_extents")
    }

    /// Returns `true` if `self` and `other` overlap (inclusive on edges).
    pub fn intersects(&self, _other: &Self) -> bool {
        todo!("Camera2D Aabb2D::intersects")
    }
}

/// 2D orthographic camera using top-left world coordinates (`+X` right, `+Y` down)
/// mapped into WebGPU normalized device coordinates (`x: [-1, 1]`, `y: [-1, 1]`, `z: [0, 1]`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Camera2D {
    pub center: Vec2,
    pub viewport_size: Vec2,
    pub zoom: f32,
}

impl Camera2D {
    /// Creates a new `Camera2D` after validating `viewport_size` (`> 0`, finite) and `zoom` (`> 0`, finite).
    pub fn new(_center: Vec2, _viewport_size: Vec2, _zoom: f32) -> Result<Self, CoreError> {
        todo!("Camera2D::new")
    }

    /// Creates a 1:1 pixel-mapped camera covering `(0.0, 0.0)..=(width, height)` (`center = (width/2, height/2)`, `zoom = 1.0`).
    pub fn from_top_left_viewport(_width: f32, _height: f32) -> Result<Self, CoreError> {
        todo!("Camera2D::from_top_left_viewport")
    }

    /// Updates the viewport dimensions (`width > 0`, `height > 0`, finite).
    pub fn set_viewport_size(&mut self, _width: f32, _height: f32) -> Result<(), CoreError> {
        todo!("Camera2D::set_viewport_size")
    }

    /// Computes the world-space `Aabb2D` currently visible by the camera:
    /// `half = (viewport_size * 0.5) / zoom`, `min = center - half`, `max = center + half`.
    pub fn visible_bounds(&self) -> Aabb2D {
        todo!("Camera2D::visible_bounds")
    }

    /// Computes the WebGPU orthographic view-projection matrix:
    /// `Mat4::orthographic_rh(bounds.min.x, bounds.max.x, bounds.max.y, bounds.min.y, -1.0, 1.0)`.
    pub fn projection_matrix(&self) -> Mat4 {
        todo!("Camera2D::projection_matrix")
    }

    /// Tests whether an object at `center` with `half_extents` is visible inside `visible_bounds()`.
    /// When `rotated` is `true`, expands half-extents conservatively to `Vec2::splat(half_extents.length())`.
    pub fn is_visible(&self, _center: Vec2, _half_extents: Vec2, _rotated: bool) -> bool {
        todo!("Camera2D::is_visible")
    }
}
