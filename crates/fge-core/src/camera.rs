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
    #[inline]
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Creates an `Aabb2D` from a center point and non-negative half-extents.
    #[inline]
    pub fn from_center_half_extents(center: Vec2, half_extents: Vec2) -> Self {
        let half = half_extents.abs();
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Returns `true` if `self` and `other` overlap (inclusive on edges).
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
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
    pub fn new(center: Vec2, viewport_size: Vec2, zoom: f32) -> Result<Self, CoreError> {
        if !center.is_finite() {
            return Err(CoreError::InvalidDimensions {
                width: center.x,
                height: center.y,
            });
        }
        if !viewport_size.is_finite() || viewport_size.x <= 0.0 || viewport_size.y <= 0.0 {
            return Err(CoreError::InvalidDimensions {
                width: viewport_size.x,
                height: viewport_size.y,
            });
        }
        if !zoom.is_finite() || zoom <= 0.0 {
            return Err(CoreError::InvalidZoom { zoom });
        }
        Ok(Self {
            center,
            viewport_size,
            zoom,
        })
    }

    /// Creates a 1:1 pixel-mapped camera covering `(0.0, 0.0)..=(width, height)` (`center = (width/2, height/2)`, `zoom = 1.0`).
    pub fn from_top_left_viewport(width: f32, height: f32) -> Result<Self, CoreError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(CoreError::InvalidDimensions { width, height });
        }
        Self::new(
            Vec2::new(width * 0.5, height * 0.5),
            Vec2::new(width, height),
            1.0,
        )
    }

    /// Updates the viewport dimensions (`width > 0`, `height > 0`, finite).
    pub fn set_viewport_size(&mut self, width: f32, height: f32) -> Result<(), CoreError> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(CoreError::InvalidDimensions { width, height });
        }
        self.viewport_size = Vec2::new(width, height);
        Ok(())
    }

    /// Computes the world-space `Aabb2D` currently visible by the camera:
    /// `half = (viewport_size * 0.5) / zoom`, `min = center - half`, `max = center + half`.
    #[inline]
    pub fn visible_bounds(&self) -> Aabb2D {
        let half = (self.viewport_size * 0.5) / self.zoom;
        Aabb2D::new(self.center - half, self.center + half)
    }

    /// Computes the WebGPU orthographic view-projection matrix:
    /// `Mat4::orthographic_rh(bounds.min.x, bounds.max.x, bounds.max.y, bounds.min.y, -1.0, 1.0)`.
    #[inline]
    pub fn projection_matrix(&self) -> Mat4 {
        let b = self.visible_bounds();
        glam::camera::rh::proj::directx::orthographic(b.min.x, b.max.x, b.max.y, b.min.y, -1.0, 1.0)
    }

    /// Tests whether an object at `center` with `half_extents` is visible inside `visible_bounds()`.
    /// When `rotated` is `true`, expands half-extents conservatively to `Vec2::splat(half_extents.length())`.
    #[inline]
    pub fn is_visible(&self, center: Vec2, half_extents: Vec2, rotated: bool) -> bool {
        let he = if rotated {
            Vec2::splat(half_extents.length())
        } else {
            half_extents.abs()
        };
        self.visible_bounds()
            .intersects(&Aabb2D::new(center - he, center + he))
    }
}
