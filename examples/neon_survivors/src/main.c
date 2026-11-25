#include "neon_shared.h"

#include <math.h>

#define MAX_BG_ENTITIES 200u

GameState g_game;

static FgeEntity g_bg_entities[MAX_BG_ENTITIES];
static uint32_t g_bg_count = 0u;

static float clamp_f32(float v, float lo, float hi) {
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

static void push_bg_entity(float x, float y, float w, float h, float rot, float z, FgeColor col) {
    if (g_bg_count >= MAX_BG_ENTITIES) {
        return;
    }
    FgeEntity e = fge_spawn_sprite(x, y, w, h, col);
    fge_set_rotation(e, rot);
    fge_set_z_order(e, z);
    fge_set_tag(e, TAG_BG_GRID);
    fge_set_flags(e, FGE_FLAG_VISIBLE);
    g_bg_entities[g_bg_count++] = e;
}

static void build_arena_background(void) {
    g_bg_count = 0u;
    for (uint32_t i = 0; i < MAX_BG_ENTITIES; ++i) {
        g_bg_entities[i] = fge_entity_null();
    }

    float step = 200.0f;
    uint32_t lines = 13u; /* 0, 200, ..., 2400 */

    for (uint32_t i = 0; i < lines; ++i) {
        float coord = (float)i * step;
        bool major = (i % 3u == 0u);
        FgeColor lcol = major ? fge_rgba(36, 78, 130, 130) : fge_rgba(20, 42, 76, 95);
        float thickness = major ? 3.0f : 1.5f;
        /* Vertical line */
        push_bg_entity(coord, NEON_ARENA_HALF, thickness, NEON_ARENA_SIZE, 0.0f, -10.0f, lcol);
        /* Horizontal line */
        push_bg_entity(NEON_ARENA_HALF, coord, NEON_ARENA_SIZE, thickness, 0.0f, -10.0f, lcol);
    }

    /* Subtle neon diamond markers at each 200x200 cell center (12 x 12 = 144) */
    for (uint32_t gy = 0; gy < 12u; ++gy) {
        for (uint32_t gx = 0; gx < 12u; ++gx) {
            float cx = ((float)gx + 0.5f) * step;
            float cy = ((float)gy + 0.5f) * step;
            bool accent = ((gx + gy) % 2u == 0u);
            FgeColor ncol = accent ? fge_rgba(32, 95, 135, 70) : fge_rgba(52, 42, 115, 65);
            push_bg_entity(cx, cy, 12.0f, 12.0f, NEON_PI * 0.25f, -10.0f, ncol);
        }
    }

    /* Arena center geometric emblem */
    push_bg_entity(NEON_ARENA_HALF, NEON_ARENA_HALF, 180.0f, 3.0f, NEON_PI * 0.25f, -9.5f, fge_rgba(65, 185, 240, 120));
    push_bg_entity(NEON_ARENA_HALF, NEON_ARENA_HALF, 180.0f, 3.0f, -NEON_PI * 0.25f, -9.5f, fge_rgba(65, 185, 240, 120));
    push_bg_entity(NEON_ARENA_HALF, NEON_ARENA_HALF, 120.0f, 4.0f, 0.0f, -9.5f, fge_rgba(90, 220, 255, 110));
    push_bg_entity(NEON_ARENA_HALF, NEON_ARENA_HALF, 4.0f, 120.0f, 0.0f, -9.5f, fge_rgba(90, 220, 255, 110));

    /* Bright neon arena perimeter walls */
    FgeColor wall_col = fge_rgba(65, 225, 255, 220);
    push_bg_entity(NEON_ARENA_HALF, 8.0f, NEON_ARENA_SIZE, 12.0f, 0.0f, -8.0f, wall_col);
    push_bg_entity(NEON_ARENA_HALF, NEON_ARENA_SIZE - 8.0f, NEON_ARENA_SIZE, 12.0f, 0.0f, -8.0f, wall_col);
    push_bg_entity(8.0f, NEON_ARENA_HALF, 12.0f, NEON_ARENA_SIZE, 0.0f, -8.0f, wall_col);
    push_bg_entity(NEON_ARENA_SIZE - 8.0f, NEON_ARENA_HALF, 12.0f, NEON_ARENA_SIZE, 0.0f, -8.0f, wall_col);
}

static void shutdown_arena_background(void) {
    for (uint32_t i = 0; i < MAX_BG_ENTITIES; ++i) {
        if (fge_entity_is_alive(g_bg_entities[i])) {
            fge_entity_despawn(g_bg_entities[i]);
        }
        g_bg_entities[i] = fge_entity_null();
    }
    g_bg_count = 0u;
}

void game_add_score(uint32_t points) {
    g_game.score += points;
}

void game_add_camera_shake(float amount) {
    g_game.shake_trauma += amount;
    if (g_game.shake_trauma > 1.0f) {
        g_game.shake_trauma = 1.0f;
    }
}

void game_sync_persisted_params(void) {
    fge_param_set(PARAM_PERSIST_SCORE, (float)g_game.score);
    fge_param_set(PARAM_PERSIST_LEVEL, (float)g_game.player_level);
    fge_param_set(PARAM_PERSIST_XP, g_game.xp);
    fge_param_set(PARAM_PERSIST_KILLS, (float)g_game.enemies_killed);
    fge_param_set(PARAM_PERSIST_WAVE, (float)g_game.wave_index);
}

static void reset_game_state(float diff_mult) {
    g_game.initialized = true;
    g_game.elapsed = 0.0f;
    g_game.difficulty_mult = diff_mult;

    g_game.score = 0u;
    g_game.player_level = 1u;
    g_game.xp = 0.0f;
    g_game.xp_next = 45.0f;
    g_game.enemies_killed = 0u;
    g_game.wave_index = 1u;

    g_game.player_hp = 100.0f;
    g_game.player_max_hp = 100.0f;
    g_game.player_invuln = 0.8f;
    g_game.player_hit_flash = 0.0f;
    g_game.player_pos = fge_vec2(NEON_ARENA_HALF, NEON_ARENA_HALF);
    g_game.player_aim_dir = fge_vec2(0.0f, -1.0f);
    g_game.autopilot_active = true;
    g_game.dash_ready_frac = 1.0f;
    g_game.nova_ready_frac = 0.75f;

    g_game.boss_active = false;
    g_game.boss_hp = 0.0f;
    g_game.boss_max_hp = 720.0f;
    g_game.boss_timer = 0.0f;

    g_game.cam_x = NEON_ARENA_HALF;
    g_game.cam_y = NEON_ARENA_HALF;
    g_game.cam_zoom = 1.0f;
    g_game.shake_trauma = 0.0f;
    g_game.shake_time = 0.0f;
}

void fge_init(void) {
    float mult = clamp_f32(fge_param_get(PARAM_DIFFICULTY_MULT, 1.0f), 0.25f, 4.0f);
    reset_game_state(mult);

    fge_camera_set(g_game.cam_x, g_game.cam_y, g_game.cam_zoom);
    build_arena_background();

    fx_init();
    player_init(NEON_ARENA_HALF, NEON_ARENA_HALF, 100.0f);
    weapons_init();
    enemies_init();
    hud_init();

    game_sync_persisted_params();
}

void fge_on_reload(void) {
    float mult = clamp_f32(fge_param_get(PARAM_DIFFICULTY_MULT, 1.0f), 0.25f, 4.0f);

    if (!g_game.initialized) {
        /* Fresh .so loaded during live hot-reload: recover player & persisted progress */
        FgeVec2 saved_pos = fge_vec2(NEON_ARENA_HALF, NEON_ARENA_HALF);
        float saved_hp = 100.0f;

        uint32_t pn = fge_query_circle(NEON_ARENA_HALF, NEON_ARENA_HALF, NEON_ARENA_SIZE * 2.0f, LAYER_PLAYER);
        if (pn > 0u) {
            FgeEntity old_p = fge_get_query_entity(0u);
            if (fge_entity_is_alive(old_p)) {
                saved_pos = fge_get_position(old_p);
                saved_hp = fge_get_hp(old_p);
            }
        }

        uint32_t all_collidables =
            LAYER_PLAYER | LAYER_ENEMY | LAYER_PLAYER_BULLET | LAYER_ENEMY_BULLET | LAYER_PICKUP;
        uint32_t cn = fge_query_circle(NEON_ARENA_HALF, NEON_ARENA_HALF, NEON_ARENA_SIZE * 2.0f, all_collidables);
        for (uint32_t i = 0; i < cn; ++i) {
            FgeEntity ce = fge_get_query_entity(i);
            if (fge_entity_is_alive(ce)) {
                fge_entity_despawn(ce);
            }
        }

        reset_game_state(mult);
        g_game.score = (uint32_t)fge_param_get(PARAM_PERSIST_SCORE, 0.0f);
        uint32_t lvl = (uint32_t)fge_param_get(PARAM_PERSIST_LEVEL, 1.0f);
        if (lvl < 1u) lvl = 1u;
        if (lvl > NEON_MAX_LEVEL) lvl = NEON_MAX_LEVEL;
        g_game.player_level = lvl;
        g_game.xp = fge_param_get(PARAM_PERSIST_XP, 0.0f);
        g_game.xp_next = 45.0f + (float)lvl * 28.0f;
        g_game.enemies_killed = (uint32_t)fge_param_get(PARAM_PERSIST_KILLS, 0.0f);
        uint32_t wv = (uint32_t)fge_param_get(PARAM_PERSIST_WAVE, 1.0f);
        g_game.wave_index = (wv > 0u) ? wv : 1u;
        g_game.cam_x = saved_pos.x;
        g_game.cam_y = saved_pos.y;

        fge_camera_set(g_game.cam_x, g_game.cam_y, g_game.cam_zoom);
        if (fge_entity_count() < 10u) {
            build_arena_background();
        }
        fx_init();
        player_init(saved_pos.x, saved_pos.y, saved_hp);
        weapons_init();
        enemies_init();
        hud_init();
    } else {
        g_game.difficulty_mult = mult;
    }

    /* Celebratory hot-reload particle ring */
    FgeVec2 ppos = player_get_position();
    fx_spawn_ring(ppos.x, ppos.y, 24u, 280.0f, 10.0f, fge_rgba(120, 255, 220, 255));
    fx_spawn_ring(ppos.x, ppos.y, 16u, 190.0f, 8.0f, fge_rgba(255, 225, 110, 255));
    game_add_camera_shake(0.22f);
}

static void handle_ai_events(void) {
    uint32_t ev_n = fge_poll_ai_events();
    for (uint32_t i = 0; i < ev_n; ++i) {
        FgeAiEvent ev = fge_get_ai_event(i);
        if (ev.kind == AI_EVENT_SPAWN_BOSS) {
            enemies_spawn_boss();
        } else if (ev.kind == AI_EVENT_NOVA_HEAL) {
            weapons_trigger_nova();
            player_heal_full();
        } else if (ev.kind == AI_EVENT_XP_SHOWER) {
            FgeVec2 ppos = player_get_position();
            float sx = (ev.arg0 > 1.0f) ? clamp_f32(ev.arg0, NEON_ARENA_MARGIN, NEON_ARENA_SIZE - NEON_ARENA_MARGIN) : ppos.x;
            float sy = (ev.arg1 > 1.0f) ? clamp_f32(ev.arg1, NEON_ARENA_MARGIN, NEON_ARENA_SIZE - NEON_ARENA_MARGIN) : ppos.y;
            weapons_spawn_xp_shower(sx, sy, 16u);
        }
    }
}

static void update_camera(float dt) {
    FgeVec2 target = player_get_position();
    float follow = 1.0f - expf(-9.0f * dt);
    g_game.cam_x += (target.x - g_game.cam_x) * follow;
    g_game.cam_y += (target.y - g_game.cam_y) * follow;

    FgeVec2 wheel = fge_mouse_wheel();
    if (wheel.y != 0.0f) {
        g_game.cam_zoom = clamp_f32(g_game.cam_zoom + wheel.y * 0.08f, 0.72f, 1.45f);
    }

    float shake_x = 0.0f;
    float shake_y = 0.0f;
    if (g_game.shake_trauma > 0.0f) {
        g_game.shake_time += dt * 42.0f;
        float mag = g_game.shake_trauma * g_game.shake_trauma * 18.0f;
        shake_x = sinf(g_game.shake_time * 1.3f) * mag;
        shake_y = cosf(g_game.shake_time * 1.7f) * mag;
        g_game.shake_trauma -= dt * 2.1f;
        if (g_game.shake_trauma < 0.0f) {
            g_game.shake_trauma = 0.0f;
        }
    }

    fge_camera_set(g_game.cam_x + shake_x, g_game.cam_y + shake_y, g_game.cam_zoom);
}

void fge_update(float dt) {
    if (dt <= 0.0f) {
        return;
    }
    if (dt > 0.05f) {
        dt = 0.05f;
    }

    g_game.elapsed += dt;
    g_game.difficulty_mult = clamp_f32(fge_param_get(PARAM_DIFFICULTY_MULT, 1.0f), 0.25f, 4.0f);

    handle_ai_events();

    player_update(dt);
    weapons_update(dt);
    enemies_update(dt);
    fx_update(dt);

    /* Update camera BEFORE hud_update so HUD reads final frame camera */
    update_camera(dt);
    hud_update(dt);

    game_sync_persisted_params();

    /* Telemetry metrics */
    fge_log_metric(METRIC_SCORE, (float)g_game.score);
    fge_log_metric(METRIC_PLAYER_LEVEL, (float)g_game.player_level);
    fge_log_metric(METRIC_ENEMIES_KILLED, (float)g_game.enemies_killed);
    fge_log_metric(METRIC_ENTITY_COUNT, (float)fge_entity_count());
}

void fge_shutdown(void) {
    hud_shutdown();
    enemies_shutdown();
    weapons_shutdown();
    player_shutdown();
    fx_shutdown();
    shutdown_arena_background();
    g_game.initialized = false;
}
