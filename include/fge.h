/*
 * fge.h - Fast Game Engine (FGE) C11 Public API Header
 *
 * Design Guarantees (MODEL.md & PROJECT.md):
 * 1. ZERO raw pointers cross the C-ABI boundary (100% value-type handles & structs).
 * 2. Every entity access is validated on the Rust side via Generational Handles (FgeEntity).
 * 3. Zero-initialized FgeEntity ({0, 0}) is guaranteed invalid (live generations >= 1).
 */

#ifndef FGE_H
#define FGE_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ==========================================================================
 * 1. Core Value Types (ABI-matched with Rust #[repr(C)])
 * ========================================================================== */

typedef struct FgeEntity {
    uint32_t index;
    uint32_t generation;
} FgeEntity;

typedef struct FgeVec2 {
    float x;
    float y;
} FgeVec2;

typedef struct FgeColor {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t a;
} FgeColor;

typedef struct FgeUvRect {
    uint16_t u0;
    uint16_t v0;
    uint16_t u1;
    uint16_t v1;
} FgeUvRect;

typedef struct FgeCamera {
    float center_x;
    float center_y;
    float viewport_w;
    float viewport_h;
    float zoom;
} FgeCamera;

typedef struct FgeCollisionPair {
    FgeEntity a;
    FgeEntity b;
} FgeCollisionPair;

typedef struct FgeAiEvent {
    uint32_t kind;
    FgeEntity target;
    float arg0;
    float arg1;
} FgeAiEvent;

typedef struct FgeSound {
    uint32_t index;
    uint32_t generation;
} FgeSound;

typedef struct FgeEmitter {
    uint32_t index;
    uint32_t generation;
} FgeEmitter;

typedef struct FgeTexture {
    uint32_t index;
    uint32_t generation;
} FgeTexture;

typedef struct FgeTextBuf {
    uint8_t bytes[120];
    uint32_t len;
} FgeTextBuf;

/* ==========================================================================
 * 2. Entity Flags & Key Codes
 * ========================================================================== */

enum {
    FGE_FLAG_VISIBLE    = 1u << 0,
    FGE_FLAG_COLLIDABLE = 1u << 1,
    FGE_FLAG_KINEMATIC  = 1u << 2,
    FGE_FLAG_Y_SORT     = 1u << 3
};

typedef enum FgeKey {
    FGE_KEY_UNKNOWN = 0,
    FGE_KEY_UP      = 1,
    FGE_KEY_DOWN    = 2,
    FGE_KEY_LEFT    = 3,
    FGE_KEY_RIGHT   = 4,
    FGE_KEY_W       = 5,
    FGE_KEY_A       = 6,
    FGE_KEY_S       = 7,
    FGE_KEY_D       = 8,
    FGE_KEY_Z       = 9,
    FGE_KEY_X       = 10,
    FGE_KEY_C       = 11,
    FGE_KEY_SPACE   = 12,
    FGE_KEY_LSHIFT  = 13,
    FGE_KEY_RSHIFT  = 14,
    FGE_KEY_ENTER   = 15,
    FGE_KEY_ESCAPE  = 16,
    FGE_KEY_0       = 17,
    FGE_KEY_1       = 18,
    FGE_KEY_2       = 19,
    FGE_KEY_3       = 20,
    FGE_KEY_4       = 21,
    FGE_KEY_5       = 22,
    FGE_KEY_6       = 23,
    FGE_KEY_7       = 24,
    FGE_KEY_8       = 25,
    FGE_KEY_9       = 26,
    FGE_KEY_Q       = 27,
    FGE_KEY_E       = 28,
    FGE_KEY_R       = 29,
    FGE_KEY_F       = 30,
    FGE_KEY_TAB     = 31,
    FGE_KEY_COUNT   = 32
} FgeKey;

typedef enum FgeMouseButton {
    FGE_MOUSE_LEFT   = 0,
    FGE_MOUSE_RIGHT  = 1,
    FGE_MOUSE_MIDDLE = 2
} FgeMouseButton;

typedef enum FgeSynthPreset {
    FGE_SFX_SLASH     = 0,
    FGE_SFX_HIT       = 1,
    FGE_SFX_EXPLOSION = 2,
    FGE_SFX_PICKUP    = 3,
    FGE_SFX_BLIP      = 4,
    FGE_SFX_CONFIRM   = 5,
    FGE_SFX_HEAL      = 6,
    FGE_SFX_LEVEL_UP  = 7
} FgeSynthPreset;

typedef enum FgeBgmPreset {
    FGE_BGM_NONE    = 0,
    FGE_BGM_VILLAGE = 1,
    FGE_BGM_FIELD   = 2,
    FGE_BGM_BATTLE  = 3,
    FGE_BGM_BOSS    = 4
} FgeBgmPreset;

typedef enum FgeWaveform {
    FGE_WAVE_SQUARE   = 0,
    FGE_WAVE_TRIANGLE = 1,
    FGE_WAVE_SAW      = 2,
    FGE_WAVE_NOISE    = 3
} FgeWaveform;

/* ==========================================================================
 * 3. Inline Value Constructors
 * ========================================================================== */

static inline FgeEntity fge_entity_null(void) {
    FgeEntity e = { 0xFFFFFFFFu, 0u };
    return e;
}

static inline FgeVec2 fge_vec2(float x, float y) {
    FgeVec2 v = { x, y };
    return v;
}

static inline FgeColor fge_rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    FgeColor c = { r, g, b, a };
    return c;
}

static inline FgeTextBuf fge_str(const char *utf8_literal) {
    FgeTextBuf buf = { { 0 }, 0u };
    if (!utf8_literal) {
        return buf;
    }
    while (buf.len < 120u && utf8_literal[buf.len] != '\0') {
        buf.bytes[buf.len] = (uint8_t)utf8_literal[buf.len];
        buf.len++;
    }
    return buf;
}

/* ==========================================================================
 * 4. Engine C-ABI Functions (Implemented in Rust crate `fge-abi`)
 * ========================================================================== */

/* Lifecycle & Timing */
float    fge_delta_time(void);
uint64_t fge_frame_count(void);
uint32_t fge_entity_count(void);
void     fge_request_quit(void);
float    fge_rand_f32(void);
float    fge_rand_range(float min_val, float max_val);

/* Entity Lifecycle */
FgeEntity fge_entity_spawn(void);
FgeEntity fge_spawn_sprite(float x, float y, float w, float h, FgeColor color);
bool      fge_entity_despawn(FgeEntity entity);
bool      fge_entity_is_alive(FgeEntity entity);

/* Transform & Kinematics */
bool    fge_set_position(FgeEntity entity, float x, float y);
FgeVec2 fge_get_position(FgeEntity entity);
bool    fge_set_velocity(FgeEntity entity, float vx, float vy);
FgeVec2 fge_get_velocity(FgeEntity entity);
bool    fge_set_size(FgeEntity entity, float w, float h);
FgeVec2 fge_get_size(FgeEntity entity);
bool    fge_set_rotation(FgeEntity entity, float radians);
float   fge_get_rotation(FgeEntity entity);
bool    fge_set_angular_velocity(FgeEntity entity, float radians_per_sec);

/* Appearance & Ordering */
bool     fge_set_color(FgeEntity entity, FgeColor color);
FgeColor fge_get_color(FgeEntity entity);
bool     fge_set_uv_rect(FgeEntity entity, FgeUvRect uv);
bool     fge_set_z_order(FgeEntity entity, float z_order);
bool     fge_set_flags(FgeEntity entity, uint32_t flags);
uint32_t fge_get_flags(FgeEntity entity);

/* Gameplay Metadata & Collision Broadphase/Narrowphase */
bool              fge_set_collider(FgeEntity entity, float radius, uint32_t layer_mask);
bool              fge_set_tag(FgeEntity entity, uint32_t tag);
uint32_t          fge_get_tag(FgeEntity entity);
bool              fge_set_hp(FgeEntity entity, float hp);
float             fge_get_hp(FgeEntity entity);
uint32_t          fge_detect_collisions(uint32_t mask_a, uint32_t mask_b);
FgeCollisionPair  fge_get_collision(uint32_t index);
uint32_t          fge_query_circle(float x, float y, float radius, uint32_t layer_mask);
FgeEntity         fge_get_query_entity(uint32_t index);

/* Camera & Input */
bool      fge_camera_set(float center_x, float center_y, float zoom);
FgeCamera fge_camera_get(void);
bool      fge_key_down(uint32_t key);
bool      fge_key_pressed(uint32_t key);
bool      fge_key_released(uint32_t key);
bool      fge_mouse_down(uint32_t button);
FgeVec2   fge_mouse_pos(void);
FgeVec2   fge_mouse_wheel(void);

/* AI Bridge Mailbox & Telemetry */
void       fge_param_set(uint32_t param_id, float value);
float      fge_param_get(uint32_t param_id, float default_val);
uint32_t   fge_poll_ai_events(void);
FgeAiEvent fge_get_ai_event(uint32_t index);
void       fge_log_metric(uint32_t metric_id, float value);

/* Audio (WAV, Procedural Chiptune SFX & RPG BGM) */
void     fge_audio_set_master_volume(float volume);
void     fge_audio_play_sfx(uint32_t preset, float volume, float pitch_scale);
void     fge_audio_play_tone(uint32_t waveform, float freq_start_hz, float freq_end_hz, float duration_sec, float volume);
FgeSound fge_sound_load_wav(FgeTextBuf relative_path);
bool     fge_sound_play(FgeSound sound, float volume);
void     fge_bgm_play_preset(uint32_t bgm_preset, float volume);
void     fge_bgm_stop(void);

/* Performance, Native TTL & Bulk Radial Emitters */
bool       fge_set_ttl(FgeEntity entity, float ttl_seconds);
FgeEmitter fge_emitter_create_radial(float x, float y, float rate_per_sec, uint32_t count_per_burst, float speed, float bullet_ttl, float radius, uint32_t layer_mask, FgeColor color);
bool       fge_emitter_set_position(FgeEmitter emitter, float x, float y);
bool       fge_emitter_set_spin(FgeEmitter emitter, float base_angle_rad, float angular_vel_rad);
bool       fge_emitter_set_active(FgeEmitter emitter, bool active);
bool       fge_emitter_destroy(FgeEmitter emitter);

/* Textures (PNG Atlas) */
FgeTexture fge_texture_load_png(FgeTextBuf relative_path);
bool       fge_set_texture(FgeEntity entity, FgeTexture texture);

/* Immediate-Mode Screen & World Text/UI */
void fge_ui_panel(float x, float y, float w, float h, FgeColor bg_color, FgeColor border_color, float border_px);
void fge_ui_bar(float x, float y, float w, float h, float ratio, FgeColor fill_color, FgeColor bg_color);
void fge_ui_text(float x, float y, float font_size, FgeColor color, FgeTextBuf text);
void fge_ui_number(float x, float y, float font_size, int64_t value, FgeColor color);
void fge_ui_dialog_box(float x, float y, float w, float h, FgeTextBuf speaker, FgeTextBuf message, uint32_t visible_chars);
void fge_world_text(float world_x, float world_y, float font_size, FgeColor color, FgeTextBuf text);

/* Game Module Entrypoints (Implemented by user C code) */
void fge_init(void);
void fge_update(float dt);
/* Optional hot-reload hook: HotReloader only hot-reloads C modules that export fge_on_reload. */
void fge_on_reload(void);
void fge_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif /* FGE_H */
