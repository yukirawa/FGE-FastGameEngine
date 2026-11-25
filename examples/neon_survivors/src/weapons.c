#include "neon_shared.h"

#include <math.h>

typedef struct BulletSlot {
    FgeEntity entity;
    float age;
    float max_life;
} BulletSlot;

typedef struct MissileSlot {
    FgeEntity entity;
    float age;
    float max_life;
    float speed;
} MissileSlot;

typedef struct GemSlot {
    FgeEntity entity;
    float value;
    float age;
} GemSlot;

static BulletSlot g_bullets[MAX_PLAYER_BULLETS];
static MissileSlot g_missiles[MAX_HOMING_MISSILES];
static FgeEntity g_blades[MAX_ORBITAL_BLADES];
static FgeEntity g_nova_segs[MAX_NOVA_SEGMENTS];
static GemSlot g_gems[MAX_XP_GEMS];

static uint32_t g_bullet_cursor = 0u;
static uint32_t g_missile_cursor = 0u;
static uint32_t g_gem_cursor = 0u;

static float g_laser_cd = 0.0f;
static float g_missile_cd = 0.0f;
static float g_blade_angle = 0.0f;
static float g_nova_anim_timer = 0.0f;
static FgeVec2 g_nova_origin = { NEON_ARENA_HALF, NEON_ARENA_HALF };

static uint32_t target_blade_count(uint32_t level) {
    uint32_t count = 3u + (level - 1u) / 2u;
    if (count > MAX_ORBITAL_BLADES) {
        count = MAX_ORBITAL_BLADES;
    }
    return count;
}

static void spawn_laser(float x, float y, float dir_x, float dir_y, float speed, float dmg, FgeColor col) {
    uint32_t chosen = MAX_PLAYER_BULLETS;
    for (uint32_t n = 0; n < MAX_PLAYER_BULLETS; ++n) {
        uint32_t idx = (g_bullet_cursor + n) % MAX_PLAYER_BULLETS;
        if (!fge_entity_is_alive(g_bullets[idx].entity)) {
            chosen = idx;
            break;
        }
    }
    if (chosen == MAX_PLAYER_BULLETS) {
        chosen = g_bullet_cursor % MAX_PLAYER_BULLETS;
        if (fge_entity_is_alive(g_bullets[chosen].entity)) {
            fge_entity_despawn(g_bullets[chosen].entity);
        }
    }
    g_bullet_cursor = (chosen + 1u) % MAX_PLAYER_BULLETS;

    FgeEntity b = fge_spawn_sprite(x, y, 18.0f, 5.5f, col);
    fge_set_velocity(b, dir_x * speed, dir_y * speed);
    fge_set_rotation(b, atan2f(dir_y, dir_x));
    fge_set_collider(b, 7.0f, LAYER_PLAYER_BULLET);
    fge_set_hp(b, dmg);
    fge_set_tag(b, TAG_PLAYER_BULLET);
    fge_set_z_order(b, 2.0f);

    g_bullets[chosen].entity = b;
    g_bullets[chosen].age = 0.0f;
    g_bullets[chosen].max_life = 1.25f;
}

static void spawn_missile(float x, float y, float angle, float dmg) {
    uint32_t chosen = MAX_HOMING_MISSILES;
    for (uint32_t n = 0; n < MAX_HOMING_MISSILES; ++n) {
        uint32_t idx = (g_missile_cursor + n) % MAX_HOMING_MISSILES;
        if (!fge_entity_is_alive(g_missiles[idx].entity)) {
            chosen = idx;
            break;
        }
    }
    if (chosen == MAX_HOMING_MISSILES) {
        chosen = g_missile_cursor % MAX_HOMING_MISSILES;
        if (fge_entity_is_alive(g_missiles[chosen].entity)) {
            fge_entity_despawn(g_missiles[chosen].entity);
        }
    }
    g_missile_cursor = (chosen + 1u) % MAX_HOMING_MISSILES;

    float speed = 520.0f + (float)g_game.player_level * 18.0f;
    FgeEntity m = fge_spawn_sprite(x, y, 16.0f, 8.0f, fge_rgba(255, 115, 240, 255));
    fge_set_velocity(m, cosf(angle) * speed, sinf(angle) * speed);
    fge_set_rotation(m, angle);
    fge_set_collider(m, 9.0f, LAYER_PLAYER_BULLET);
    fge_set_hp(m, dmg);
    fge_set_tag(m, TAG_PLAYER_MISSILE);
    fge_set_z_order(m, 3.0f);

    g_missiles[chosen].entity = m;
    g_missiles[chosen].age = 0.0f;
    g_missiles[chosen].max_life = 2.6f;
    g_missiles[chosen].speed = speed;
}

void weapons_grant_xp(float amount) {
    if (amount <= 0.0f) {
        return;
    }
    g_game.xp += amount;
    while (g_game.xp >= g_game.xp_next && g_game.player_level < NEON_MAX_LEVEL) {
        g_game.xp -= g_game.xp_next;
        g_game.player_level += 1u;
        g_game.xp_next = 45.0f + (float)g_game.player_level * 28.0f;
        g_game.player_hp += 15.0f;
        if (g_game.player_hp > g_game.player_max_hp) {
            g_game.player_hp = g_game.player_max_hp;
        }
        FgeEntity p = player_get_entity();
        if (fge_entity_is_alive(p)) {
            fge_set_hp(p, g_game.player_hp);
        }
        FgeVec2 pos = player_get_position();
        fx_spawn_ring(pos.x, pos.y, 24u, 260.0f, 9.0f, fge_rgba(80, 255, 220, 255));
        fx_spawn_ring(pos.x, pos.y, 16u, 170.0f, 7.0f, fge_rgba(255, 220, 80, 255));
        game_add_camera_shake(0.25f);
        game_add_score(250u * g_game.player_level);
    }
    if (g_game.player_level >= NEON_MAX_LEVEL && g_game.xp > g_game.xp_next) {
        g_game.xp = g_game.xp_next;
    }
}

void weapons_spawn_xp_gem(float x, float y, float value) {
    uint32_t chosen = MAX_XP_GEMS;
    for (uint32_t n = 0; n < MAX_XP_GEMS; ++n) {
        uint32_t idx = (g_gem_cursor + n) % MAX_XP_GEMS;
        if (!fge_entity_is_alive(g_gems[idx].entity)) {
            chosen = idx;
            break;
        }
    }
    if (chosen == MAX_XP_GEMS) {
        chosen = g_gem_cursor % MAX_XP_GEMS;
        if (fge_entity_is_alive(g_gems[chosen].entity)) {
            fge_entity_despawn(g_gems[chosen].entity);
        }
    }
    g_gem_cursor = (chosen + 1u) % MAX_XP_GEMS;

    bool high_val = (value >= 12.0f);
    float sz = high_val ? 13.0f : 9.5f;
    FgeColor col = high_val ? fge_rgba(90, 255, 190, 255) : fge_rgba(65, 235, 255, 240);

    FgeEntity g = fge_spawn_sprite(x, y, sz, sz, col);
    fge_set_rotation(g, NEON_PI * 0.25f);
    fge_set_angular_velocity(g, 2.5f);
    fge_set_collider(g, 11.0f, LAYER_PICKUP);
    fge_set_tag(g, TAG_XP_GEM);
    fge_set_z_order(g, 1.5f);
    fge_set_flags(g, fge_get_flags(g) | FGE_FLAG_Y_SORT);

    g_gems[chosen].entity = g;
    g_gems[chosen].value = value;
    g_gems[chosen].age = 0.0f;
}

void weapons_spawn_xp_shower(float x, float y, uint32_t count) {
    for (uint32_t i = 0; i < count; ++i) {
        float angle = (NEON_TAU * (float)i) / (float)(count > 0u ? count : 1u);
        float dist = fge_rand_range(60.0f, 160.0f);
        float gx = x + cosf(angle) * dist;
        float gy = y + sinf(angle) * dist;
        if (gx < NEON_ARENA_MARGIN) gx = NEON_ARENA_MARGIN;
        if (gx > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) gx = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        if (gy < NEON_ARENA_MARGIN) gy = NEON_ARENA_MARGIN;
        if (gy > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) gy = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        weapons_spawn_xp_gem(gx, gy, 10.0f);
    }
    fx_spawn_ring(x, y, 18u, 200.0f, 7.0f, fge_rgba(80, 255, 210, 255));
}

void weapons_trigger_nova(void) {
    FgeVec2 pos = player_get_position();
    g_nova_origin = pos;
    g_nova_anim_timer = 0.55f;

    /* Clear enemy bullets in huge radius */
    enemies_clear_bullets_in_radius(pos.x, pos.y, 720.0f);

    /* Damage all enemies within shockwave radius */
    float nova_dmg = 70.0f + (float)g_game.player_level * 18.0f;
    uint32_t hit_n = fge_query_circle(pos.x, pos.y, 580.0f, LAYER_ENEMY);
    for (uint32_t i = 0; i < hit_n; ++i) {
        FgeEntity e = fge_get_query_entity(i);
        if (!fge_entity_is_alive(e)) {
            continue;
        }
        float hp = fge_get_hp(e) - nova_dmg;
        fge_set_hp(e, hp);
        FgeVec2 ep = fge_get_position(e);
        fx_spawn_hit_spark(ep.x, ep.y, fge_rgba(255, 200, 255, 255));
    }

    for (uint32_t i = 0; i < MAX_NOVA_SEGMENTS; ++i) {
        if (!fge_entity_is_alive(g_nova_segs[i])) {
            g_nova_segs[i] = fge_spawn_sprite(pos.x, pos.y, 44.0f, 10.0f, fge_rgba(255, 160, 255, 255));
            fge_set_tag(g_nova_segs[i], TAG_PARTICLE);
            fge_set_z_order(g_nova_segs[i], 15.0f);
        }
        fge_set_flags(g_nova_segs[i], FGE_FLAG_VISIBLE);
    }

    fx_spawn_ring(pos.x, pos.y, 28u, 360.0f, 10.0f, fge_rgba(255, 140, 255, 255));
    game_add_camera_shake(0.55f);
}

void weapons_init(void) {
    g_bullet_cursor = 0u;
    g_missile_cursor = 0u;
    g_gem_cursor = 0u;
    g_laser_cd = 0.0f;
    g_missile_cd = 0.4f;
    g_blade_angle = 0.0f;
    g_nova_anim_timer = 0.0f;

    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        g_bullets[i].entity = fge_entity_null();
        g_bullets[i].age = 0.0f;
        g_bullets[i].max_life = 1.0f;
    }
    for (uint32_t i = 0; i < MAX_HOMING_MISSILES; ++i) {
        g_missiles[i].entity = fge_entity_null();
        g_missiles[i].age = 0.0f;
        g_missiles[i].max_life = 2.5f;
        g_missiles[i].speed = 500.0f;
    }
    for (uint32_t i = 0; i < MAX_XP_GEMS; ++i) {
        g_gems[i].entity = fge_entity_null();
        g_gems[i].value = 5.0f;
        g_gems[i].age = 0.0f;
    }

    for (uint32_t i = 0; i < MAX_ORBITAL_BLADES; ++i) {
        g_blades[i] = fge_spawn_sprite(NEON_ARENA_HALF, NEON_ARENA_HALF, 22.0f, 8.0f, fge_rgba(110, 255, 200, 255));
        fge_set_collider(g_blades[i], 13.0f, LAYER_PLAYER_BULLET);
        fge_set_hp(g_blades[i], 14.0f);
        fge_set_tag(g_blades[i], TAG_PLAYER_BLADE);
        fge_set_z_order(g_blades[i], 4.0f);
        fge_set_flags(g_blades[i], FGE_FLAG_VISIBLE | FGE_FLAG_COLLIDABLE);
    }

    for (uint32_t i = 0; i < MAX_NOVA_SEGMENTS; ++i) {
        g_nova_segs[i] = fge_spawn_sprite(NEON_ARENA_HALF, NEON_ARENA_HALF, 44.0f, 10.0f, fge_rgba(255, 160, 255, 0));
        fge_set_tag(g_nova_segs[i], TAG_PARTICLE);
        fge_set_z_order(g_nova_segs[i], 15.0f);
        fge_set_flags(g_nova_segs[i], 0u);
    }
}

void weapons_update(float dt) {
    FgeVec2 ppos = player_get_position();
    uint32_t lvl = g_game.player_level;
    if (lvl < 1u) lvl = 1u;
    if (lvl > NEON_MAX_LEVEL) lvl = NEON_MAX_LEVEL;

    /* 1. Pulse Laser Firing */
    g_laser_cd -= dt;
    if (g_laser_cd <= 0.0f) {
        float fire_period = 0.145f - (float)(lvl - 1u) * 0.0085f;
        if (fire_period < 0.062f) {
            fire_period = 0.062f;
        }
        g_laser_cd = fire_period;

        float base_angle = atan2f(g_game.player_aim_dir.y, g_game.player_aim_dir.x);
        bool focus = !g_game.autopilot_active &&
                     (fge_key_down(FGE_KEY_LSHIFT) || fge_key_down(FGE_KEY_RSHIFT));
        float bullet_speed = 1080.0f + (float)lvl * 32.0f;
        float bullet_dmg = 12.0f + (float)lvl * 2.2f;

        uint32_t count = 2u;
        if (lvl >= 8u) {
            count = 5u;
        } else if (lvl >= 5u) {
            count = 4u;
        } else if (lvl >= 3u) {
            count = 3u;
        }

        float spread = focus ? 0.045f : 0.11f;
        FgeColor lcol = (lvl >= 7u)
                            ? fge_rgba(255, 240, 120, 255)
                            : ((lvl >= 4u) ? fge_rgba(120, 255, 240, 255) : fge_rgba(140, 235, 255, 255));

        if (count == 2u) {
            float px = -sinf(base_angle) * 9.0f;
            float py = cosf(base_angle) * 9.0f;
            float dx = cosf(base_angle);
            float dy = sinf(base_angle);
            spawn_laser(ppos.x + px, ppos.y + py, dx, dy, bullet_speed, bullet_dmg, lcol);
            spawn_laser(ppos.x - px, ppos.y - py, dx, dy, bullet_speed, bullet_dmg, lcol);
        } else {
            float half = 0.5f * (float)(count - 1u);
            for (uint32_t i = 0; i < count; ++i) {
                float off = ((float)i - half) * spread;
                float a = base_angle + off;
                spawn_laser(
                    ppos.x + cosf(a) * 16.0f,
                    ppos.y + sinf(a) * 16.0f,
                    cosf(a),
                    sinf(a),
                    bullet_speed,
                    bullet_dmg,
                    lcol
                );
            }
        }
    }

    /* 2. Homing Missiles */
    g_missile_cd -= dt;
    if (g_missile_cd <= 0.0f) {
        float m_period = 1.05f - (float)(lvl - 1u) * 0.065f;
        if (m_period < 0.42f) {
            m_period = 0.42f;
        }
        g_missile_cd = m_period;

        uint32_t salvo = 1u + (lvl >= 4u ? 1u : 0u) + (lvl >= 7u ? 1u : 0u);
        float base_a = atan2f(g_game.player_aim_dir.y, g_game.player_aim_dir.x);
        float m_dmg = 24.0f + (float)lvl * 4.5f;
        for (uint32_t i = 0; i < salvo; ++i) {
            float side = ((float)i - 0.5f * (float)(salvo - 1u)) * 0.65f + ((i % 2u == 0u) ? 0.35f : -0.35f);
            spawn_missile(ppos.x, ppos.y, base_a + side, m_dmg);
        }
    }

    /* 3. Update & Guide Homing Missiles via fge_query_circle */
    for (uint32_t i = 0; i < MAX_HOMING_MISSILES; ++i) {
        MissileSlot *ms = &g_missiles[i];
        if (!fge_entity_is_alive(ms->entity)) {
            ms->entity = fge_entity_null();
            continue;
        }
        ms->age += dt;
        FgeVec2 mpos = fge_get_position(ms->entity);
        if (ms->age >= ms->max_life ||
            mpos.x < -64.0f || mpos.x > NEON_ARENA_SIZE + 64.0f ||
            mpos.y < -64.0f || mpos.y > NEON_ARENA_SIZE + 64.0f) {
            fge_entity_despawn(ms->entity);
            ms->entity = fge_entity_null();
            continue;
        }

        uint32_t n_near = fge_query_circle(mpos.x, mpos.y, 560.0f, LAYER_ENEMY);
        float best_d2 = 1e18f;
        FgeVec2 target = mpos;
        bool has_target = false;
        for (uint32_t k = 0; k < n_near; ++k) {
            FgeEntity e = fge_get_query_entity(k);
            if (!fge_entity_is_alive(e)) {
                continue;
            }
            FgeVec2 ep = fge_get_position(e);
            float dx = ep.x - mpos.x;
            float dy = ep.y - mpos.y;
            float d2 = dx * dx + dy * dy;
            if (d2 < best_d2) {
                best_d2 = d2;
                target = ep;
                has_target = true;
            }
        }

        FgeVec2 vel = fge_get_velocity(ms->entity);
        float cur_angle = atan2f(vel.y, vel.x);
        if (has_target) {
            float desired = atan2f(target.y - mpos.y, target.x - mpos.x);
            float diff = desired - cur_angle;
            while (diff > NEON_PI) diff -= NEON_TAU;
            while (diff < -NEON_PI) diff += NEON_TAU;
            float max_turn = 7.5f * dt;
            if (diff > max_turn) diff = max_turn;
            if (diff < -max_turn) diff = -max_turn;
            cur_angle += diff;
        }

        float vx = cosf(cur_angle) * ms->speed;
        float vy = sinf(cur_angle) * ms->speed;
        fge_set_velocity(ms->entity, vx, vy);
        fge_set_rotation(ms->entity, cur_angle);

        if ((uint32_t)(ms->age * 30.0f) != (uint32_t)((ms->age - dt) * 30.0f)) {
            fx_spawn_particle(
                mpos.x - cosf(cur_angle) * 8.0f,
                mpos.y - sinf(cur_angle) * 8.0f,
                -vx * 0.15f,
                -vy * 0.15f,
                5.0f,
                1.0f,
                0.18f,
                4.0f,
                fge_rgba(255, 130, 235, 190)
            );
        }
    }

    /* 4. Prune Player Laser Bullets */
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        BulletSlot *bs = &g_bullets[i];
        if (!fge_entity_is_alive(bs->entity)) {
            bs->entity = fge_entity_null();
            continue;
        }
        bs->age += dt;
        FgeVec2 bp = fge_get_position(bs->entity);
        if (bs->age >= bs->max_life ||
            bp.x < -40.0f || bp.x > NEON_ARENA_SIZE + 40.0f ||
            bp.y < -40.0f || bp.y > NEON_ARENA_SIZE + 40.0f) {
            fge_entity_despawn(bs->entity);
            bs->entity = fge_entity_null();
        }
    }

    /* 5. Update Orbital Blades */
    uint32_t active_blades = target_blade_count(lvl);
    float blade_spin_speed = 3.4f + (float)lvl * 0.35f;
    float orbit_r = 82.0f + (float)lvl * 2.5f;
    g_blade_angle += dt * blade_spin_speed;

    for (uint32_t i = 0; i < MAX_ORBITAL_BLADES; ++i) {
        if (!fge_entity_is_alive(g_blades[i])) {
            continue;
        }
        if (i < active_blades) {
            float a = g_blade_angle + (NEON_TAU * (float)i) / (float)active_blades;
            float bx = ppos.x + cosf(a) * orbit_r;
            float by = ppos.y + sinf(a) * orbit_r;
            fge_set_position(g_blades[i], bx, by);
            fge_set_rotation(g_blades[i], a + NEON_PI * 0.5f);
            fge_set_hp(g_blades[i], (22.0f + (float)lvl * 5.0f) * dt * 12.0f);
            fge_set_flags(g_blades[i], FGE_FLAG_VISIBLE | FGE_FLAG_COLLIDABLE);
            fge_set_color(
                g_blades[i],
                (lvl >= 6u) ? fge_rgba(255, 230, 100, 255) : fge_rgba(110, 255, 200, 255)
            );
        } else {
            fge_set_flags(g_blades[i], 0u);
        }
    }

    /* 6. Update Nova Shockwave Ring Visuals */
    if (g_nova_anim_timer > 0.0f) {
        g_nova_anim_timer -= dt;
        float t = 1.0f - (g_nova_anim_timer / 0.55f);
        if (t < 0.0f) t = 0.0f;
        if (t > 1.0f) t = 1.0f;
        float r = 30.0f + t * 560.0f;
        uint8_t alpha = (uint8_t)((1.0f - t) * 240.0f);
        for (uint32_t i = 0; i < MAX_NOVA_SEGMENTS; ++i) {
            if (!fge_entity_is_alive(g_nova_segs[i])) {
                continue;
            }
            if (g_nova_anim_timer <= 0.0f) {
                fge_set_flags(g_nova_segs[i], 0u);
            } else {
                float a = (NEON_TAU * (float)i) / (float)MAX_NOVA_SEGMENTS + t * 0.8f;
                fge_set_position(
                    g_nova_segs[i],
                    g_nova_origin.x + cosf(a) * r,
                    g_nova_origin.y + sinf(a) * r
                );
                fge_set_rotation(g_nova_segs[i], a + NEON_PI * 0.5f);
                fge_set_size(g_nova_segs[i], 46.0f + t * 90.0f, 10.0f * (1.0f - 0.5f * t));
                fge_set_color(g_nova_segs[i], fge_rgba(255, 150, 255, alpha));
            }
        }
    }

    /* 7. Magnetize XP Gems via fge_query_circle */
    float magnet_r = 160.0f + (float)lvl * 16.0f;
    uint32_t mag_n = fge_query_circle(ppos.x, ppos.y, magnet_r, LAYER_PICKUP);
    for (uint32_t i = 0; i < mag_n; ++i) {
        FgeEntity g = fge_get_query_entity(i);
        if (!fge_entity_is_alive(g)) {
            continue;
        }
        FgeVec2 gp = fge_get_position(g);
        float dx = ppos.x - gp.x;
        float dy = ppos.y - gp.y;
        float d2 = dx * dx + dy * dy;
        if (d2 > 1.0f) {
            float dist = sqrtf(d2);
            float pull_spd = 420.0f + (magnet_r - dist) * 1.8f;
            fge_set_velocity(g, (dx / dist) * pull_spd, (dy / dist) * pull_spd);
        }
    }

    /* 8. Collect XP Gems via fge_detect_collisions */
    uint32_t pickup_hits = fge_detect_collisions(LAYER_PLAYER, LAYER_PICKUP);
    for (uint32_t i = 0; i < pickup_hits; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.b)) {
            continue;
        }
        float val = 8.0f;
        for (uint32_t k = 0; k < MAX_XP_GEMS; ++k) {
            if (g_gems[k].entity.index == pair.b.index &&
                g_gems[k].entity.generation == pair.b.generation) {
                val = g_gems[k].value;
                g_gems[k].entity = fge_entity_null();
                break;
            }
        }
        FgeVec2 gp = fge_get_position(pair.b);
        fge_entity_despawn(pair.b);
        fx_spawn_hit_spark(gp.x, gp.y, fge_rgba(80, 255, 220, 255));
        weapons_grant_xp(val);
        game_add_score(25u);
    }
}

void weapons_shutdown(void) {
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        if (fge_entity_is_alive(g_bullets[i].entity)) {
            fge_entity_despawn(g_bullets[i].entity);
        }
        g_bullets[i].entity = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_HOMING_MISSILES; ++i) {
        if (fge_entity_is_alive(g_missiles[i].entity)) {
            fge_entity_despawn(g_missiles[i].entity);
        }
        g_missiles[i].entity = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_ORBITAL_BLADES; ++i) {
        if (fge_entity_is_alive(g_blades[i])) {
            fge_entity_despawn(g_blades[i]);
        }
        g_blades[i] = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_NOVA_SEGMENTS; ++i) {
        if (fge_entity_is_alive(g_nova_segs[i])) {
            fge_entity_despawn(g_nova_segs[i]);
        }
        g_nova_segs[i] = fge_entity_null();
    }
    for (uint32_t i = 0; i < MAX_XP_GEMS; ++i) {
        if (fge_entity_is_alive(g_gems[i].entity)) {
            fge_entity_despawn(g_gems[i].entity);
        }
        g_gems[i].entity = fge_entity_null();
    }
}
