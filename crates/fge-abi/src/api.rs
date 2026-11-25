//! Exported `extern "C"` host functions matching Section 4 of `include/fge.h`.
//!
//! Guarantees:
//! - Zero raw pointers cross the C-ABI boundary.
//! - Every entity handle is validated through `EntityWorld`'s generational arena.
//! - Calls made outside `with_active_context` or with stale handles / out-of-bounds indices
//!   return safe default values without panicking.

use fge_audio::synth::{BgmPreset, SfxPreset, Waveform};
use fge_core::{
    camera::Camera2D,
    emitter::EmitterSpec,
    handle::EntityId,
    input_core::{FgeKey, FgeMouseButton},
    spatial::CollisionPair,
};
use glam::Vec2;

use crate::{
    context::try_with_context_mut,
    types::{
        FgeAiEvent, FgeCamera, FgeCollisionPair, FgeColor, FgeEmitter, FgeEntity, FgeSound,
        FgeTextBuf, FgeTexture, FgeUvRect, FgeVec2,
    },
};

const INVALID_COLLISION_PAIR: FgeCollisionPair = CollisionPair {
    a: EntityId::INVALID,
    b: EntityId::INVALID,
};

// ============================================================================
// Lifecycle & Timing
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_delta_time() -> f32 {
    try_with_context_mut(|ctx| ctx.dt).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn fge_frame_count() -> u64 {
    try_with_context_mut(|ctx| ctx.frame_count).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_entity_count() -> u32 {
    try_with_context_mut(|ctx| ctx.world.len().min(u32::MAX as usize) as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_request_quit() {
    let _ = try_with_context_mut(|ctx| {
        ctx.quit_requested = true;
    });
}

#[no_mangle]
pub extern "C" fn fge_rand_f32() -> f32 {
    try_with_context_mut(|ctx| ctx.rng.f32()).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn fge_rand_range(min_val: f32, max_val: f32) -> f32 {
    if !min_val.is_finite() || !max_val.is_finite() {
        return 0.0;
    }
    try_with_context_mut(|ctx| {
        if min_val >= max_val {
            return min_val;
        }
        let t = ctx.rng.f32() as f64;
        let span = (max_val as f64) - (min_val as f64);
        ((min_val as f64) + span * t) as f32
    })
    .unwrap_or(0.0)
}

// ============================================================================
// Entity Lifecycle
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_entity_spawn() -> FgeEntity {
    try_with_context_mut(|ctx| ctx.world.spawn()).unwrap_or(EntityId::INVALID)
}

#[no_mangle]
pub extern "C" fn fge_spawn_sprite(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: FgeColor,
) -> FgeEntity {
    try_with_context_mut(|ctx| {
        ctx.world
            .spawn_sprite([x, y], [w, h], [color.r, color.g, color.b, color.a])
    })
    .unwrap_or(EntityId::INVALID)
}

#[no_mangle]
pub extern "C" fn fge_entity_despawn(entity: FgeEntity) -> bool {
    try_with_context_mut(|ctx| ctx.world.despawn(entity)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_entity_is_alive(entity: FgeEntity) -> bool {
    try_with_context_mut(|ctx| ctx.world.is_alive(entity)).unwrap_or(false)
}

// ============================================================================
// Transform & Kinematics
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_set_position(entity: FgeEntity, x: f32, y: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_position(entity, [x, y])).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_position(entity: FgeEntity) -> FgeVec2 {
    try_with_context_mut(|ctx| {
        ctx.world
            .get_position(entity)
            .map(|[x, y]| FgeVec2 { x, y })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_set_velocity(entity: FgeEntity, vx: f32, vy: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_velocity(entity, [vx, vy])).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_velocity(entity: FgeEntity) -> FgeVec2 {
    try_with_context_mut(|ctx| {
        ctx.world
            .get_velocity(entity)
            .map(|[x, y]| FgeVec2 { x, y })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_set_size(entity: FgeEntity, w: f32, h: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_size(entity, [w, h])).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_size(entity: FgeEntity) -> FgeVec2 {
    try_with_context_mut(|ctx| {
        ctx.world
            .get_size(entity)
            .map(|[x, y]| FgeVec2 { x, y })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_set_rotation(entity: FgeEntity, radians: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_rotation(entity, radians)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_rotation(entity: FgeEntity) -> f32 {
    try_with_context_mut(|ctx| ctx.world.get_rotation(entity).unwrap_or(0.0)).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn fge_set_angular_velocity(entity: FgeEntity, radians_per_sec: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_angular_velocity(entity, radians_per_sec))
        .unwrap_or(false)
}

// ============================================================================
// Appearance & Ordering
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_set_color(entity: FgeEntity, color: FgeColor) -> bool {
    try_with_context_mut(|ctx| {
        ctx.world
            .set_color(entity, [color.r, color.g, color.b, color.a])
    })
    .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_color(entity: FgeEntity) -> FgeColor {
    try_with_context_mut(|ctx| {
        ctx.world
            .get_color(entity)
            .map(|[r, g, b, a]| FgeColor { r, g, b, a })
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_set_uv_rect(entity: FgeEntity, uv: FgeUvRect) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_uv_rect(entity, [uv.u0, uv.v0, uv.u1, uv.v1]))
        .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_set_z_order(entity: FgeEntity, z_order: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_z_order(entity, z_order)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_set_flags(entity: FgeEntity, flags: u32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_flags(entity, flags)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_flags(entity: FgeEntity) -> u32 {
    try_with_context_mut(|ctx| ctx.world.get_flags(entity).unwrap_or(0)).unwrap_or(0)
}

// ============================================================================
// Gameplay Metadata & Collision Broadphase/Narrowphase
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_set_collider(entity: FgeEntity, radius: f32, layer_mask: u32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_collider(entity, radius, layer_mask)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_set_tag(entity: FgeEntity, tag: u32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_tag(entity, tag)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_tag(entity: FgeEntity) -> u32 {
    try_with_context_mut(|ctx| ctx.world.get_tag(entity).unwrap_or(0)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_set_hp(entity: FgeEntity, hp: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_hp(entity, hp)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_get_hp(entity: FgeEntity) -> f32 {
    try_with_context_mut(|ctx| ctx.world.get_hp(entity).unwrap_or(0.0)).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn fge_detect_collisions(mask_a: u32, mask_b: u32) -> u32 {
    try_with_context_mut(|ctx| {
        ctx.last_collisions.clear();
        let count = ctx.spatial.detect_collisions(
            &ctx.world,
            mask_a,
            mask_b,
            &mut ctx.last_collisions,
        );
        count.min(u32::MAX as usize) as u32
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_get_collision(index: u32) -> FgeCollisionPair {
    try_with_context_mut(|ctx| {
        ctx.last_collisions
            .get(index as usize)
            .copied()
            .unwrap_or(INVALID_COLLISION_PAIR)
    })
    .unwrap_or(INVALID_COLLISION_PAIR)
}

#[no_mangle]
pub extern "C" fn fge_query_circle(x: f32, y: f32, radius: f32, layer_mask: u32) -> u32 {
    try_with_context_mut(|ctx| {
        ctx.last_query_entities.clear();
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius < 0.0 || layer_mask == 0
        {
            return 0;
        }
        ctx.spatial.rebuild(&ctx.world);
        let count = ctx.spatial.query_circle(
            &ctx.world,
            Vec2::new(x, y),
            radius,
            layer_mask,
            &mut ctx.last_query_entities,
        );
        count.min(u32::MAX as usize) as u32
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_get_query_entity(index: u32) -> FgeEntity {
    try_with_context_mut(|ctx| {
        ctx.last_query_entities
            .get(index as usize)
            .copied()
            .unwrap_or(EntityId::INVALID)
    })
    .unwrap_or(EntityId::INVALID)
}

// ============================================================================
// Camera & Input
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_camera_set(center_x: f32, center_y: f32, zoom: f32) -> bool {
    try_with_context_mut(|ctx| {
        match Camera2D::new(Vec2::new(center_x, center_y), ctx.camera.viewport_size, zoom) {
            Ok(camera) => {
                ctx.camera = camera;
                true
            }
            Err(_) => false,
        }
    })
    .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_camera_get() -> FgeCamera {
    try_with_context_mut(|ctx| FgeCamera {
        center_x: ctx.camera.center.x,
        center_y: ctx.camera.center.y,
        viewport_w: ctx.camera.viewport_size.x,
        viewport_h: ctx.camera.viewport_size.y,
        zoom: ctx.camera.zoom,
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_key_down(key: u32) -> bool {
    let Some(key) = FgeKey::from_u32(key) else {
        return false;
    };
    try_with_context_mut(|ctx| ctx.input.is_key_down(key)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_key_pressed(key: u32) -> bool {
    let Some(key) = FgeKey::from_u32(key) else {
        return false;
    };
    try_with_context_mut(|ctx| ctx.input.is_key_pressed(key)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_key_released(key: u32) -> bool {
    let Some(key) = FgeKey::from_u32(key) else {
        return false;
    };
    try_with_context_mut(|ctx| ctx.input.is_key_released(key)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_mouse_down(button: u32) -> bool {
    let Some(button) = FgeMouseButton::from_u32(button) else {
        return false;
    };
    try_with_context_mut(|ctx| ctx.input.is_mouse_down(button)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_mouse_pos() -> FgeVec2 {
    try_with_context_mut(|ctx| FgeVec2 {
        x: ctx.input.mouse_pos[0],
        y: ctx.input.mouse_pos[1],
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_mouse_wheel() -> FgeVec2 {
    try_with_context_mut(|ctx| FgeVec2 {
        x: ctx.input.mouse_wheel[0],
        y: ctx.input.mouse_wheel[1],
    })
    .unwrap_or_default()
}

// ============================================================================
// AI Bridge Mailbox & Telemetry
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_param_set(param_id: u32, value: f32) {
    let _ = try_with_context_mut(|ctx| ctx.set_param(param_id, value));
}

#[no_mangle]
pub extern "C" fn fge_param_get(param_id: u32, default_val: f32) -> f32 {
    try_with_context_mut(|ctx| ctx.get_param(param_id, default_val)).unwrap_or(default_val)
}

#[no_mangle]
pub extern "C" fn fge_poll_ai_events() -> u32 {
    try_with_context_mut(|ctx| ctx.poll_ai_events()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn fge_get_ai_event(index: u32) -> FgeAiEvent {
    try_with_context_mut(|ctx| {
        ctx.active_ai_events
            .get(index as usize)
            .copied()
            .unwrap_or_default()
    })
    .unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn fge_log_metric(metric_id: u32, value: f32) {
    let _ = try_with_context_mut(|ctx| ctx.record_metric(metric_id, value));
}

// ============================================================================
// Audio (WAV, Procedural Chiptune SFX & RPG BGM)
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_audio_set_master_volume(volume: f32) {
    let _ = try_with_context_mut(|ctx| ctx.audio.set_master_volume(volume));
}

#[no_mangle]
pub extern "C" fn fge_audio_play_sfx(preset: u32, volume: f32, pitch_scale: f32) {
    let Some(preset) = SfxPreset::from_u32(preset) else {
        return;
    };
    let _ = try_with_context_mut(|ctx| ctx.audio.play_sfx(preset, volume, pitch_scale));
}

#[no_mangle]
pub extern "C" fn fge_audio_play_tone(
    waveform: u32,
    freq_start_hz: f32,
    freq_end_hz: f32,
    duration_sec: f32,
    volume: f32,
) {
    let Some(waveform) = Waveform::from_u32(waveform) else {
        return;
    };
    let _ = try_with_context_mut(|ctx| {
        ctx.audio
            .play_tone(waveform, freq_start_hz, freq_end_hz, duration_sec, volume)
    });
}

#[no_mangle]
pub extern "C" fn fge_sound_load_wav(relative_path: FgeTextBuf) -> FgeSound {
    try_with_context_mut(|ctx| {
        let rel = relative_path.as_str();
        let trimmed = rel.trim();
        if trimmed.is_empty() {
            return EntityId::INVALID;
        }
        let full_path = ctx.resolve_asset_path(trimmed);
        ctx.audio
            .load_wav_file(&full_path)
            .unwrap_or(EntityId::INVALID)
    })
    .unwrap_or(EntityId::INVALID)
}

#[no_mangle]
pub extern "C" fn fge_sound_play(sound: FgeSound, volume: f32) -> bool {
    try_with_context_mut(|ctx| ctx.audio.play_sound(sound, volume)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_bgm_play_preset(bgm_preset: u32, volume: f32) {
    let Some(preset) = BgmPreset::from_u32(bgm_preset) else {
        return;
    };
    let _ = try_with_context_mut(|ctx| ctx.audio.play_bgm_preset(preset, volume));
}

#[no_mangle]
pub extern "C" fn fge_bgm_stop() {
    let _ = try_with_context_mut(|ctx| ctx.audio.stop_bgm());
}

// ============================================================================
// Performance, Native TTL & Bulk Radial Emitters
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_set_ttl(entity: FgeEntity, ttl_seconds: f32) -> bool {
    try_with_context_mut(|ctx| ctx.world.set_ttl(entity, ttl_seconds)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_emitter_create_radial(
    x: f32,
    y: f32,
    rate_per_sec: f32,
    count_per_burst: u32,
    speed: f32,
    bullet_ttl: f32,
    radius: f32,
    layer_mask: u32,
    color: FgeColor,
) -> FgeEmitter {
    try_with_context_mut(|ctx| {
        let diameter = if radius.is_finite() && radius > 0.0 {
            radius * 2.0
        } else {
            8.0
        };
        let spec = EmitterSpec {
            pos: [x, y],
            rate_per_sec,
            count_per_burst,
            speed,
            bullet_ttl,
            collider_radius: radius,
            layer_mask,
            size: [diameter, diameter],
            color_rgba: [color.r, color.g, color.b, color.a],
            base_angle_rad: 0.0,
            angular_vel_rad: 0.0,
            active: true,
        };
        ctx.emitters.create_radial(spec)
    })
    .unwrap_or(EntityId::INVALID)
}

#[no_mangle]
pub extern "C" fn fge_emitter_set_position(emitter: FgeEmitter, x: f32, y: f32) -> bool {
    try_with_context_mut(|ctx| ctx.emitters.set_position(emitter, x, y)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_emitter_set_spin(
    emitter: FgeEmitter,
    base_angle_rad: f32,
    angular_vel_rad: f32,
) -> bool {
    try_with_context_mut(|ctx| {
        ctx.emitters
            .set_spin(emitter, base_angle_rad, angular_vel_rad)
    })
    .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_emitter_set_active(emitter: FgeEmitter, active: bool) -> bool {
    try_with_context_mut(|ctx| ctx.emitters.set_active(emitter, active)).unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn fge_emitter_destroy(emitter: FgeEmitter) -> bool {
    try_with_context_mut(|ctx| ctx.emitters.destroy(emitter)).unwrap_or(false)
}

// ============================================================================
// Textures (PNG Atlas)
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_texture_load_png(relative_path: FgeTextBuf) -> FgeTexture {
    try_with_context_mut(|ctx| {
        let rel = relative_path.as_str();
        ctx.queue_texture_load_png(rel.as_ref())
    })
    .unwrap_or(EntityId::INVALID)
}

#[no_mangle]
pub extern "C" fn fge_set_texture(entity: FgeEntity, texture: FgeTexture) -> bool {
    try_with_context_mut(|ctx| ctx.bind_entity_texture(entity, texture)).unwrap_or(false)
}

// ============================================================================
// Immediate-Mode Screen & World Text/UI
// ============================================================================

#[no_mangle]
pub extern "C" fn fge_ui_panel(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    bg_color: FgeColor,
    border_color: FgeColor,
    border_px: f32,
) {
    let _ = try_with_context_mut(|ctx| {
        ctx.ui.push_panel(
            x,
            y,
            w,
            h,
            [bg_color.r, bg_color.g, bg_color.b, bg_color.a],
            [
                border_color.r,
                border_color.g,
                border_color.b,
                border_color.a,
            ],
            border_px,
        );
    });
}

#[no_mangle]
pub extern "C" fn fge_ui_bar(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    ratio: f32,
    fill_color: FgeColor,
    bg_color: FgeColor,
) {
    let _ = try_with_context_mut(|ctx| {
        ctx.ui.push_bar(
            x,
            y,
            w,
            h,
            ratio,
            [fill_color.r, fill_color.g, fill_color.b, fill_color.a],
            [bg_color.r, bg_color.g, bg_color.b, bg_color.a],
        );
    });
}

#[no_mangle]
pub extern "C" fn fge_ui_text(
    x: f32,
    y: f32,
    font_size: f32,
    color: FgeColor,
    text: FgeTextBuf,
) {
    let _ = try_with_context_mut(|ctx| {
        let s = text.as_str();
        ctx.ui
            .push_screen_text(x, y, font_size, [color.r, color.g, color.b, color.a], s.as_ref());
    });
}

#[no_mangle]
pub extern "C" fn fge_ui_number(
    x: f32,
    y: f32,
    font_size: f32,
    value: i64,
    color: FgeColor,
) {
    let _ = try_with_context_mut(|ctx| {
        ctx.ui
            .push_number(x, y, font_size, value, [color.r, color.g, color.b, color.a]);
    });
}

#[no_mangle]
pub extern "C" fn fge_ui_dialog_box(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    speaker: FgeTextBuf,
    message: FgeTextBuf,
    visible_chars: u32,
) {
    let _ = try_with_context_mut(|ctx| {
        let spk = speaker.as_str();
        let msg = message.as_str();
        ctx.ui
            .push_dialog_box(x, y, w, h, spk.as_ref(), msg.as_ref(), visible_chars);
    });
}

#[no_mangle]
pub extern "C" fn fge_world_text(
    world_x: f32,
    world_y: f32,
    font_size: f32,
    color: FgeColor,
    text: FgeTextBuf,
) {
    let _ = try_with_context_mut(|ctx| {
        let s = text.as_str();
        ctx.ui.push_world_text(
            world_x,
            world_y,
            font_size,
            [color.r, color.g, color.b, color.a],
            s.as_ref(),
        );
    });
}

/// Forces the linker to retain all 68 exported `fge_*` C-ABI symbols so `dlopen`-loaded
/// game modules can resolve them against host binaries linked with `-Wl,--export-dynamic`.
pub fn ensure_abi_symbols_linked() -> usize {
    let symbols: [usize; 68] = [
        // Lifecycle & Timing (6)
        fge_delta_time as *const () as usize,
        fge_frame_count as *const () as usize,
        fge_entity_count as *const () as usize,
        fge_request_quit as *const () as usize,
        fge_rand_f32 as *const () as usize,
        fge_rand_range as *const () as usize,
        // Entity Lifecycle (4)
        fge_entity_spawn as *const () as usize,
        fge_spawn_sprite as *const () as usize,
        fge_entity_despawn as *const () as usize,
        fge_entity_is_alive as *const () as usize,
        // Transform & Kinematics (9)
        fge_set_position as *const () as usize,
        fge_get_position as *const () as usize,
        fge_set_velocity as *const () as usize,
        fge_get_velocity as *const () as usize,
        fge_set_size as *const () as usize,
        fge_get_size as *const () as usize,
        fge_set_rotation as *const () as usize,
        fge_get_rotation as *const () as usize,
        fge_set_angular_velocity as *const () as usize,
        // Appearance & Ordering (6)
        fge_set_color as *const () as usize,
        fge_get_color as *const () as usize,
        fge_set_uv_rect as *const () as usize,
        fge_set_z_order as *const () as usize,
        fge_set_flags as *const () as usize,
        fge_get_flags as *const () as usize,
        // Gameplay Metadata & Collision Broadphase/Narrowphase (9)
        fge_set_collider as *const () as usize,
        fge_set_tag as *const () as usize,
        fge_get_tag as *const () as usize,
        fge_set_hp as *const () as usize,
        fge_get_hp as *const () as usize,
        fge_detect_collisions as *const () as usize,
        fge_get_collision as *const () as usize,
        fge_query_circle as *const () as usize,
        fge_get_query_entity as *const () as usize,
        // Camera & Input (8)
        fge_camera_set as *const () as usize,
        fge_camera_get as *const () as usize,
        fge_key_down as *const () as usize,
        fge_key_pressed as *const () as usize,
        fge_key_released as *const () as usize,
        fge_mouse_down as *const () as usize,
        fge_mouse_pos as *const () as usize,
        fge_mouse_wheel as *const () as usize,
        // AI Bridge Mailbox & Telemetry (5)
        fge_param_set as *const () as usize,
        fge_param_get as *const () as usize,
        fge_poll_ai_events as *const () as usize,
        fge_get_ai_event as *const () as usize,
        fge_log_metric as *const () as usize,
        // Audio (7)
        fge_audio_set_master_volume as *const () as usize,
        fge_audio_play_sfx as *const () as usize,
        fge_audio_play_tone as *const () as usize,
        fge_sound_load_wav as *const () as usize,
        fge_sound_play as *const () as usize,
        fge_bgm_play_preset as *const () as usize,
        fge_bgm_stop as *const () as usize,
        // Performance, Native TTL & Bulk Radial Emitters (6)
        fge_set_ttl as *const () as usize,
        fge_emitter_create_radial as *const () as usize,
        fge_emitter_set_position as *const () as usize,
        fge_emitter_set_spin as *const () as usize,
        fge_emitter_set_active as *const () as usize,
        fge_emitter_destroy as *const () as usize,
        // Textures (2)
        fge_texture_load_png as *const () as usize,
        fge_set_texture as *const () as usize,
        // Immediate-Mode Screen & World Text/UI (6)
        fge_ui_panel as *const () as usize,
        fge_ui_bar as *const () as usize,
        fge_ui_text as *const () as usize,
        fge_ui_number as *const () as usize,
        fge_ui_dialog_box as *const () as usize,
        fge_world_text as *const () as usize,
    ];
    std::hint::black_box(symbols).len()
}
