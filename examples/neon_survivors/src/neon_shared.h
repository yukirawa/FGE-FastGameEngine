#ifndef NEON_SHARED_H
#define NEON_SHARED_H

#include <stdbool.h>
#include <stdint.h>

#include "fge.h"

/* ==========================================================================
 * Arena, Math & Cap Constants
 * ========================================================================== */

#define NEON_VIEW_W          1280.0f
#define NEON_VIEW_H          720.0f
#define NEON_ARENA_SIZE      2400.0f
#define NEON_ARENA_HALF      1200.0f
#define NEON_ARENA_MARGIN    42.0f
#define NEON_PI              3.14159265358979323846f
#define NEON_TAU             6.28318530717958647692f

#define NEON_MAX_LEVEL       10u
#define NEON_BOSS_INTERVAL   20.0f
#define NEON_AUTOPILOT_DELAY 3.0f

/* Bounded entity pool limits (total live entities always < 3,500) */
#define MAX_PLAYER_BULLETS   320u
#define MAX_HOMING_MISSILES  48u
#define MAX_ORBITAL_BLADES   6u
#define MAX_NOVA_SEGMENTS    24u
#define MAX_XP_GEMS          300u
#define MAX_ENEMIES          180u
#define MAX_ENEMY_BULLETS    1200u
#define MAX_PARTICLES        900u

/* ==========================================================================
 * Collision Layer Masks
 * ========================================================================== */

enum {
    LAYER_PLAYER        = 1u << 0,
    LAYER_ENEMY         = 1u << 1,
    LAYER_PLAYER_BULLET = 1u << 2,
    LAYER_ENEMY_BULLET  = 1u << 3,
    LAYER_PICKUP        = 1u << 4
};

/* ==========================================================================
 * Entity Tags
 * ========================================================================== */

enum {
    TAG_PLAYER         = 1u,
    TAG_ENEMY_CHASER   = 2u,
    TAG_ENEMY_DASHER   = 3u,
    TAG_ENEMY_SHOOTER  = 4u,
    TAG_BOSS           = 5u,
    TAG_PLAYER_BULLET  = 6u,
    TAG_PLAYER_MISSILE = 7u,
    TAG_PLAYER_BLADE   = 8u,
    TAG_ENEMY_BULLET   = 9u,
    TAG_XP_GEM         = 10u,
    TAG_PARTICLE       = 11u,
    TAG_BG_GRID        = 12u,
    TAG_HUD            = 13u,
    TAG_PLAYER_PART    = 14u,
    TAG_BOSS_PART      = 15u
};

/* ==========================================================================
 * AI Bridge Parameters, Events & Telemetry Metrics
 * ========================================================================== */

enum {
    PARAM_DIFFICULTY_MULT = 1u,
    /* Internal state persistence across hot-reload */
    PARAM_PERSIST_SCORE   = 100u,
    PARAM_PERSIST_LEVEL   = 101u,
    PARAM_PERSIST_XP      = 102u,
    PARAM_PERSIST_KILLS   = 103u,
    PARAM_PERSIST_WAVE    = 104u
};

enum {
    AI_EVENT_SPAWN_BOSS  = 1u,
    AI_EVENT_NOVA_HEAL   = 2u,
    AI_EVENT_XP_SHOWER   = 3u
};

enum {
    METRIC_SCORE          = 1u,
    METRIC_PLAYER_LEVEL   = 2u,
    METRIC_ENEMIES_KILLED = 3u,
    METRIC_ENTITY_COUNT   = 4u
};

/* ==========================================================================
 * Shared Game State
 * ========================================================================== */

typedef struct GameState {
    bool initialized;
    float elapsed;
    float difficulty_mult;

    /* Player progression & vitals */
    uint32_t score;
    uint32_t player_level;
    float xp;
    float xp_next;
    uint32_t enemies_killed;
    uint32_t wave_index;

    float player_hp;
    float player_max_hp;
    float player_invuln;
    float player_hit_flash;
    FgeVec2 player_pos;
    FgeVec2 player_aim_dir;
    bool autopilot_active;

    /* Ability cooldowns / charges (0..1 normalized readiness exposed for HUD) */
    float dash_ready_frac;
    float nova_ready_frac;

    /* Boss state for HUD & wave pacing */
    bool boss_active;
    float boss_hp;
    float boss_max_hp;
    float boss_timer;

    /* Camera & screen shake */
    float cam_x;
    float cam_y;
    float cam_zoom;
    float shake_trauma;
    float shake_time;
} GameState;

extern GameState g_game;

/* ==========================================================================
 * Module Entrypoints & Shared Helpers
 * ========================================================================== */

/* main.c */
void game_add_score(uint32_t points);
void game_add_camera_shake(float amount);
void game_sync_persisted_params(void);
void fge_on_reload(void);

/* player.c */
void player_init(float start_x, float start_y, float start_hp);
void player_update(float dt);
void player_shutdown(void);
void player_apply_damage(float dmg, FgeVec2 knock_from);
void player_heal_full(void);
FgeEntity player_get_entity(void);
FgeVec2 player_get_position(void);

/* weapons.c */
void weapons_init(void);
void weapons_update(float dt);
void weapons_shutdown(void);
void weapons_trigger_nova(void);
void weapons_spawn_xp_gem(float x, float y, float value);
void weapons_spawn_xp_shower(float x, float y, uint32_t count);
void weapons_grant_xp(float amount);

/* enemies.c */
void enemies_init(void);
void enemies_update(float dt);
void enemies_shutdown(void);
void enemies_spawn_boss(void);
void enemies_clear_bullets_in_radius(float cx, float cy, float radius);
uint32_t enemies_alive_count(void);

/* fx.c */
void fx_init(void);
void fx_update(float dt);
void fx_shutdown(void);
void fx_spawn_particle(
    float x,
    float y,
    float vx,
    float vy,
    float start_size,
    float end_size,
    float lifetime,
    float ang_vel,
    FgeColor color
);
void fx_spawn_burst(float x, float y, uint32_t count, float speed_min, float speed_max, float size, FgeColor color);
void fx_spawn_hit_spark(float x, float y, FgeColor color);
void fx_spawn_ring(float x, float y, uint32_t count, float speed, float size, FgeColor color);
void fx_spawn_thruster(float x, float y, float dir_x, float dir_y, FgeColor color);

/* hud.c */
void hud_init(void);
void hud_update(float dt);
void hud_shutdown(void);

#endif /* NEON_SHARED_H */
