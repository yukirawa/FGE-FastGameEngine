#include "game_shared.h"

#include <math.h>

#define MAX_ENEMIES       16
#define MAX_ENEMY_BULLETS 256
#define ENEMIES_PER_WAVE  7

static FgeEntity g_enemies[MAX_ENEMIES];
static FgeEntity g_enemy_bullets[MAX_ENEMY_BULLETS];
static float g_enemy_elapsed = 0.0f;
static float g_aimed_timer = 0.0f;
static float g_radial_timer = 0.0f;
static uint32_t g_wave_index = 1u;

static void spawn_enemy_bullet(float x, float y, float vx, float vy, FgeColor color) {
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        if (!fge_entity_is_alive(g_enemy_bullets[i])) {
            FgeEntity b = fge_spawn_sprite(x, y, 10.0f, 10.0f, color);
            fge_set_velocity(b, vx, vy);
            fge_set_collider(b, 5.0f, LAYER_ENEMY_BULLET);
            fge_set_tag(b, TAG_BULLET);
            fge_set_z_order(b, 6.0f);
            g_enemy_bullets[i] = b;
            return;
        }
    }
}

static void emit_aimed_shot(FgeEntity shooter, FgeVec2 target) {
    FgeVec2 pos = fge_get_position(shooter);
    float dx = target.x - pos.x;
    float dy = target.y - pos.y;
    float len_sq = dx * dx + dy * dy;
    if (len_sq < 1.0f) {
        return;
    }
    float inv_len = 1.0f / sqrtf(len_sq);
    float speed = 260.0f + (float)g_wave_index * 15.0f;
    spawn_enemy_bullet(pos.x, pos.y + 18.0f, dx * inv_len * speed, dy * inv_len * speed,
                       fge_rgba(255, 190, 90, 255));
}

static void emit_radial_burst(FgeEntity shooter, uint32_t spokes, float angle_offset) {
    FgeVec2 pos = fge_get_position(shooter);
    float speed = 210.0f;
    for (uint32_t s = 0; s < spokes; ++s) {
        float angle = angle_offset + (2.0f * GAME_PI * (float)s) / (float)spokes;
        float vx = cosf(angle) * speed;
        float vy = sinf(angle) * speed;
        spawn_enemy_bullet(pos.x, pos.y, vx, vy, fge_rgba(255, 95, 165, 255));
    }
}

static void prune_offscreen_enemy_bullets(void) {
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        FgeEntity b = g_enemy_bullets[i];
        if (!fge_entity_is_alive(b)) {
            g_enemy_bullets[i] = fge_entity_null();
            continue;
        }
        FgeVec2 pos = fge_get_position(b);
        if (pos.y < -32.0f || pos.y > GAME_HEIGHT + 32.0f ||
            pos.x < -32.0f || pos.x > GAME_WIDTH + 32.0f) {
            fge_entity_despawn(b);
            g_enemy_bullets[i] = fge_entity_null();
        }
    }
}

void enemy_spawn_wave(uint32_t wave_index) {
    g_wave_index = wave_index;
    float hp = 30.0f + (float)(wave_index - 1u) * 10.0f;
    float center = GAME_WIDTH * 0.5f;
    float spacing = 120.0f;
    int32_t half = (int32_t)(ENEMIES_PER_WAVE / 2u);

    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (i < ENEMIES_PER_WAVE) {
            int32_t offset = (int32_t)i - half;
            float x = center + (float)offset * spacing;
            float y = 220.0f - (float)(offset < 0 ? -offset : offset) * 24.0f;
            FgeEntity e = fge_spawn_sprite(x, y, 36.0f, 36.0f, fge_rgba(255, 90, 110, 255));
            fge_set_hp(e, hp);
            fge_set_collider(e, 20.0f, LAYER_ENEMY);
            fge_set_tag(e, TAG_ENEMY);
            fge_set_z_order(e, 8.0f);
            fge_set_angular_velocity(e, (i % 2u == 0u) ? 0.9f : -0.9f);
            g_enemies[i] = e;
        } else {
            g_enemies[i] = fge_entity_null();
        }
    }
}

uint32_t enemy_alive_count(void) {
    uint32_t count = 0u;
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (fge_entity_is_alive(g_enemies[i])) {
            ++count;
        }
    }
    return count;
}

void enemy_init(void) {
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        g_enemies[i] = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        g_enemy_bullets[i] = fge_entity_null();
    }
    g_enemy_elapsed = 0.0f;
    g_aimed_timer = 0.1f;
    g_radial_timer = 0.2f;
    enemy_spawn_wave(1u);
}

void enemy_update(float dt) {
    g_enemy_elapsed += dt;
    g_aimed_timer -= dt;
    g_radial_timer -= dt;

    /* Sway living enemies horizontally */
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        FgeEntity e = g_enemies[i];
        if (!fge_entity_is_alive(e)) {
            g_enemies[i] = fge_entity_null();
            continue;
        }
        float phase = g_enemy_elapsed * 1.8f + (float)i * 0.55f;
        float vx = cosf(phase) * 90.0f;
        fge_set_velocity(e, vx, 0.0f);
    }

    /* Aimed bullet emission */
    if (g_aimed_timer <= 0.0f) {
        FgeVec2 target = player_get_position();
        for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
            if (fge_entity_is_alive(g_enemies[i]) && (i % 2u == 0u)) {
                emit_aimed_shot(g_enemies[i], target);
            }
        }
        g_aimed_timer = 0.55f;
    }

    /* Radial bullet burst from the center enemy */
    if (g_radial_timer <= 0.0f) {
        uint32_t center_idx = ENEMIES_PER_WAVE / 2u;
        if (fge_entity_is_alive(g_enemies[center_idx])) {
            emit_radial_burst(g_enemies[center_idx], 8u, g_enemy_elapsed);
        }
        g_radial_timer = 0.85f;
    }

    /* Detect player bullet -> enemy collisions and apply damage */
    uint32_t hit_count = fge_detect_collisions(LAYER_PLAYER_BULLET, LAYER_ENEMY);
    for (uint32_t i = 0; i < hit_count; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.a) || !fge_entity_is_alive(pair.b)) {
            continue;
        }
        fge_entity_despawn(pair.a);
        float hp = fge_get_hp(pair.b) - 10.0f;
        if (hp <= 0.0f) {
            fge_entity_despawn(pair.b);
            game_add_score(100u);
        } else {
            fge_set_hp(pair.b, hp);
        }
    }

    prune_offscreen_enemy_bullets();

    if (enemy_alive_count() == 0u) {
        enemy_spawn_wave(g_wave_index + 1u);
    }
}

void enemy_shutdown(void) {
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (fge_entity_is_alive(g_enemies[i])) {
            fge_entity_despawn(g_enemies[i]);
        }
        g_enemies[i] = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        if (fge_entity_is_alive(g_enemy_bullets[i])) {
            fge_entity_despawn(g_enemy_bullets[i]);
        }
        g_enemy_bullets[i] = fge_entity_null();
    }
}
