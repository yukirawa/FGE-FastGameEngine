#include "neon_shared.h"

#include <math.h>

typedef enum EnemyKind {
    ENEMY_CHASER  = 0,
    ENEMY_DASHER  = 1,
    ENEMY_SHOOTER = 2,
    ENEMY_BOSS    = 3
} EnemyKind;

typedef struct EnemySlot {
    FgeEntity entity;
    EnemyKind kind;
    float max_hp;
    float speed;
    float fire_timer;
    float state_timer;
    float hit_flash;
    uint32_t phase_step;
    FgeVec2 dash_vec;
    FgeColor base_color;
    bool active;
} EnemySlot;

typedef struct EnemyBulletSlot {
    FgeEntity entity;
    float age;
    float max_life;
} EnemyBulletSlot;

static EnemySlot g_enemies[MAX_ENEMIES];
static EnemyBulletSlot g_enemy_bullets[MAX_ENEMY_BULLETS];
static uint32_t g_ebullet_cursor = 0u;

/* Giant Multi-Part Boss satellite visuals */
static FgeEntity g_boss_eye;
static FgeEntity g_boss_armor[4];
static float g_boss_orbit_angle = 0.0f;
static float g_boss_spiral_angle = 0.0f;
static float g_wave_spawn_timer = 0.0f;

static int32_t find_enemy_slot_by_entity(FgeEntity e) {
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (g_enemies[i].active &&
            g_enemies[i].entity.index == e.index &&
            g_enemies[i].entity.generation == e.generation) {
            return (int32_t)i;
        }
    }
    return -1;
}

static void spawn_enemy_bullet(float x, float y, float vx, float vy, float size, FgeColor col) {
    uint32_t chosen = MAX_ENEMY_BULLETS;
    for (uint32_t n = 0; n < MAX_ENEMY_BULLETS; ++n) {
        uint32_t idx = (g_ebullet_cursor + n) % MAX_ENEMY_BULLETS;
        if (!fge_entity_is_alive(g_enemy_bullets[idx].entity)) {
            chosen = idx;
            break;
        }
    }
    if (chosen == MAX_ENEMY_BULLETS) {
        chosen = g_ebullet_cursor % MAX_ENEMY_BULLETS;
        if (fge_entity_is_alive(g_enemy_bullets[chosen].entity)) {
            fge_entity_despawn(g_enemy_bullets[chosen].entity);
        }
    }
    g_ebullet_cursor = (chosen + 1u) % MAX_ENEMY_BULLETS;

    FgeEntity b = fge_spawn_sprite(x, y, size, size, col);
    fge_set_velocity(b, vx, vy);
    fge_set_rotation(b, atan2f(vy, vx) + NEON_PI * 0.25f);
    fge_set_angular_velocity(b, 3.0f);
    fge_set_collider(b, size * 0.42f, LAYER_ENEMY_BULLET);
    fge_set_tag(b, TAG_ENEMY_BULLET);
    fge_set_z_order(b, 6.0f);

    g_enemy_bullets[chosen].entity = b;
    g_enemy_bullets[chosen].age = 0.0f;
    g_enemy_bullets[chosen].max_life = 6.5f;
}

static void emit_aimed_spread(float sx, float sy, FgeVec2 target, uint32_t ways, float spread_rad, float speed, float size, FgeColor col) {
    float base = atan2f(target.y - sy, target.x - sx);
    float half = 0.5f * (float)(ways - 1u);
    for (uint32_t i = 0; i < ways; ++i) {
        float a = base + ((float)i - half) * spread_rad;
        spawn_enemy_bullet(sx, sy, cosf(a) * speed, sinf(a) * speed, size, col);
    }
}

static void emit_radial_ring(float sx, float sy, uint32_t spokes, float angle_offset, float speed, float size, FgeColor col) {
    for (uint32_t i = 0; i < spokes; ++i) {
        float a = angle_offset + (NEON_TAU * (float)i) / (float)spokes;
        spawn_enemy_bullet(sx, sy, cosf(a) * speed, sinf(a) * speed, size, col);
    }
}

static void pick_spawn_ring_pos(FgeVec2 center, float min_r, float max_r, float *out_x, float *out_y) {
    float angle = fge_rand_range(0.0f, NEON_TAU);
    float dist = fge_rand_range(min_r, max_r);
    float x = center.x + cosf(angle) * dist;
    float y = center.y + sinf(angle) * dist;
    if (x < NEON_ARENA_MARGIN + 24.0f) x = NEON_ARENA_MARGIN + 24.0f;
    if (x > NEON_ARENA_SIZE - NEON_ARENA_MARGIN - 24.0f) x = NEON_ARENA_SIZE - NEON_ARENA_MARGIN - 24.0f;
    if (y < NEON_ARENA_MARGIN + 24.0f) y = NEON_ARENA_MARGIN + 24.0f;
    if (y > NEON_ARENA_SIZE - NEON_ARENA_MARGIN - 24.0f) y = NEON_ARENA_SIZE - NEON_ARENA_MARGIN - 24.0f;
    *out_x = x;
    *out_y = y;
}

static void spawn_single_enemy(EnemyKind kind, float x, float y) {
    int32_t free_idx = -1;
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (!g_enemies[i].active) {
            free_idx = (int32_t)i;
            break;
        }
    }
    if (free_idx < 0) {
        return;
    }

    float mult = g_game.difficulty_mult;
    float wave_scale = 1.0f + (float)(g_game.wave_index > 0u ? g_game.wave_index - 1u : 0u) * 0.16f;
    float hp = 26.0f * wave_scale * mult;
    float spd = 145.0f * mult;
    float sz_w = 26.0f;
    float sz_h = 26.0f;
    float col_r = 15.0f;
    uint32_t tag = TAG_ENEMY_CHASER;
    FgeColor col = fge_rgba(255, 70, 105, 255);

    if (kind == ENEMY_DASHER) {
        hp = 34.0f * wave_scale * mult;
        spd = 175.0f * mult;
        sz_w = 32.0f;
        sz_h = 18.0f;
        col_r = 15.0f;
        tag = TAG_ENEMY_DASHER;
        col = fge_rgba(255, 165, 55, 255);
    } else if (kind == ENEMY_SHOOTER) {
        hp = 44.0f * wave_scale * mult;
        spd = 115.0f * mult;
        sz_w = 32.0f;
        sz_h = 32.0f;
        col_r = 17.0f;
        tag = TAG_ENEMY_SHOOTER;
        col = fge_rgba(195, 85, 255, 255);
    } else if (kind == ENEMY_BOSS) {
        hp = (720.0f + (float)g_game.wave_index * 160.0f) * mult;
        spd = 92.0f * mult;
        sz_w = 74.0f;
        sz_h = 74.0f;
        col_r = 40.0f;
        tag = TAG_BOSS;
        col = fge_rgba(245, 50, 155, 255);
    }

    FgeEntity e = fge_spawn_sprite(x, y, sz_w, sz_h, col);
    fge_set_hp(e, hp);
    fge_set_collider(e, col_r, LAYER_ENEMY);
    fge_set_tag(e, tag);
    fge_set_z_order(e, (kind == ENEMY_BOSS) ? 8.5f : 7.0f);
    fge_set_flags(e, fge_get_flags(e) | FGE_FLAG_Y_SORT);
    if (kind == ENEMY_CHASER || kind == ENEMY_SHOOTER) {
        fge_set_angular_velocity(e, (kind == ENEMY_SHOOTER) ? -1.4f : 1.8f);
    }

    EnemySlot *s = &g_enemies[free_idx];
    s->entity = e;
    s->kind = kind;
    s->max_hp = hp;
    s->speed = spd;
    s->fire_timer = fge_rand_range(0.5f, 1.4f);
    s->state_timer = 0.0f;
    s->hit_flash = 0.0f;
    s->phase_step = 0u;
    s->dash_vec = fge_vec2(0.0f, 0.0f);
    s->base_color = col;
    s->active = true;

    if (kind == ENEMY_BOSS) {
        g_game.boss_active = true;
        g_game.boss_hp = hp;
        g_game.boss_max_hp = hp;
        g_game.boss_timer = 0.0f;
        g_boss_spiral_angle = 0.0f;

        if (!fge_entity_is_alive(g_boss_eye)) {
            g_boss_eye = fge_spawn_sprite(x, y, 26.0f, 26.0f, fge_rgba(255, 235, 255, 255));
            fge_set_tag(g_boss_eye, TAG_BOSS_PART);
            fge_set_z_order(g_boss_eye, 9.0f);
        }
        fge_set_flags(g_boss_eye, FGE_FLAG_VISIBLE);
        fge_set_position(g_boss_eye, x, y);

        for (uint32_t k = 0; k < 4u; ++k) {
            if (!fge_entity_is_alive(g_boss_armor[k])) {
                g_boss_armor[k] = fge_spawn_sprite(x, y, 24.0f, 24.0f, fge_rgba(140, 70, 255, 255));
                fge_set_tag(g_boss_armor[k], TAG_BOSS_PART);
                fge_set_z_order(g_boss_armor[k], 8.8f);
            }
            fge_set_flags(g_boss_armor[k], FGE_FLAG_VISIBLE);
        }
        fx_spawn_ring(x, y, 28u, 320.0f, 12.0f, fge_rgba(255, 80, 200, 255));
        game_add_camera_shake(0.5f);
    }
}

static void hide_boss_parts(void) {
    if (fge_entity_is_alive(g_boss_eye)) {
        fge_set_flags(g_boss_eye, 0u);
    }
    for (uint32_t k = 0; k < 4u; ++k) {
        if (fge_entity_is_alive(g_boss_armor[k])) {
            fge_set_flags(g_boss_armor[k], 0u);
        }
    }
    g_game.boss_active = false;
    g_game.boss_hp = 0.0f;
}

static void kill_enemy_slot(EnemySlot *s) {
    FgeVec2 ep = fge_get_position(s->entity);
    fge_entity_despawn(s->entity);
    s->entity = fge_entity_null();
    s->active = false;

    g_game.enemies_killed += 1u;

    if (s->kind == ENEMY_BOSS) {
        hide_boss_parts();
        fx_spawn_burst(ep.x, ep.y, 48u, 110.0f, 440.0f, 14.0f, fge_rgba(255, 110, 220, 255));
        fx_spawn_ring(ep.x, ep.y, 32u, 340.0f, 11.0f, fge_rgba(255, 230, 120, 255));
        weapons_spawn_xp_shower(ep.x, ep.y, 22u);
        game_add_camera_shake(0.85f);
        game_add_score(2500u);
    } else {
        uint32_t pts = (s->kind == ENEMY_SHOOTER) ? 160u : ((s->kind == ENEMY_DASHER) ? 130u : 100u);
        float xp_val = (s->kind == ENEMY_SHOOTER) ? 14.0f : 8.0f;
        fx_spawn_burst(ep.x, ep.y, 12u, 70.0f, 240.0f, 7.5f, s->base_color);
        weapons_spawn_xp_gem(ep.x, ep.y, xp_val);
        game_add_camera_shake(0.08f);
        game_add_score(pts);
    }
}

void enemies_spawn_boss(void) {
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (g_enemies[i].active && g_enemies[i].kind == ENEMY_BOSS) {
            return;
        }
    }
    FgeVec2 ppos = player_get_position();
    float bx = 0.0f;
    float by = 0.0f;
    pick_spawn_ring_pos(ppos, 380.0f, 520.0f, &bx, &by);
    spawn_single_enemy(ENEMY_BOSS, bx, by);
}

static void spawn_wave(uint32_t wave) {
    g_game.wave_index = wave;
    FgeVec2 ppos = player_get_position();

    uint32_t count = 8u + wave * 3u;
    if (count > 36u) {
        count = 36u;
    }

    for (uint32_t i = 0; i < count; ++i) {
        float x = 0.0f;
        float y = 0.0f;
        pick_spawn_ring_pos(ppos, 390.0f, 640.0f, &x, &y);

        EnemyKind kind = ENEMY_CHASER;
        if (wave >= 2u && (i % 4u == 2u)) {
            kind = ENEMY_DASHER;
        } else if (wave >= 2u && (i % 5u == 4u)) {
            kind = ENEMY_SHOOTER;
        } else if (i % 6u == 5u) {
            kind = ENEMY_SHOOTER;
        }
        spawn_single_enemy(kind, x, y);
    }

    if ((wave % 3u == 0u) && !g_game.boss_active) {
        enemies_spawn_boss();
    }
}

void enemies_clear_bullets_in_radius(float cx, float cy, float radius) {
    float r2 = radius * radius;
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        FgeEntity b = g_enemy_bullets[i].entity;
        if (!fge_entity_is_alive(b)) {
            g_enemy_bullets[i].entity = fge_entity_null();
            continue;
        }
        FgeVec2 bp = fge_get_position(b);
        float dx = bp.x - cx;
        float dy = bp.y - cy;
        if (dx * dx + dy * dy <= r2) {
            fx_spawn_particle(
                bp.x,
                bp.y,
                dx * 0.6f,
                dy * 0.6f,
                7.0f,
                1.0f,
                0.24f,
                5.0f,
                fge_rgba(255, 190, 255, 230)
            );
            fge_entity_despawn(b);
            g_enemy_bullets[i].entity = fge_entity_null();
        }
    }
}

uint32_t enemies_alive_count(void) {
    uint32_t count = 0u;
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (g_enemies[i].active && fge_entity_is_alive(g_enemies[i].entity)) {
            ++count;
        }
    }
    return count;
}

void enemies_init(void) {
    g_ebullet_cursor = 0u;
    g_boss_orbit_angle = 0.0f;
    g_boss_spiral_angle = 0.0f;
    g_wave_spawn_timer = 0.0f;
    g_game.boss_active = false;
    g_game.boss_hp = 0.0f;
    g_game.boss_max_hp = 720.0f;
    g_game.boss_timer = 0.0f;

    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        g_enemies[i].entity = fge_entity_null();
        g_enemies[i].active = false;
    }
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        g_enemy_bullets[i].entity = fge_entity_null();
        g_enemy_bullets[i].age = 0.0f;
        g_enemy_bullets[i].max_life = 6.0f;
    }

    g_boss_eye = fge_entity_null();
    for (uint32_t k = 0; k < 4u; ++k) {
        g_boss_armor[k] = fge_entity_null();
    }

    spawn_wave(g_game.wave_index > 0u ? g_game.wave_index : 1u);
}

void enemies_update(float dt) {
    FgeVec2 ppos = player_get_position();
    float mult = g_game.difficulty_mult;
    g_boss_orbit_angle += dt * 1.9f;

    if (!g_game.boss_active) {
        g_game.boss_timer += dt;
        if (g_game.boss_timer >= NEON_BOSS_INTERVAL) {
            enemies_spawn_boss();
        }
    }

    /* Update enemy AI & boss danmaku patterns */
    bool boss_found = false;
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        EnemySlot *s = &g_enemies[i];
        if (!s->active) {
            continue;
        }
        if (!fge_entity_is_alive(s->entity)) {
            if (s->kind == ENEMY_BOSS) {
                hide_boss_parts();
            }
            s->active = false;
            s->entity = fge_entity_null();
            continue;
        }

        if (fge_get_hp(s->entity) <= 0.0f) {
            kill_enemy_slot(s);
            continue;
        }

        if (s->hit_flash > 0.0f) {
            s->hit_flash -= dt;
            fge_set_color(s->entity, fge_rgba(255, 255, 255, 255));
        } else {
            fge_set_color(s->entity, s->base_color);
        }

        FgeVec2 ep = fge_get_position(s->entity);
        float dx = ppos.x - ep.x;
        float dy = ppos.y - ep.y;
        float dist = sqrtf(dx * dx + dy * dy + 1.0f);
        float dir_x = dx / dist;
        float dir_y = dy / dist;

        if (s->kind == ENEMY_CHASER) {
            fge_set_velocity(s->entity, dir_x * s->speed, dir_y * s->speed);
        } else if (s->kind == ENEMY_DASHER) {
            s->state_timer -= dt;
            if (s->phase_step == 0u) {
                /* Approach until mid-range */
                fge_set_velocity(s->entity, dir_x * s->speed, dir_y * s->speed);
                fge_set_rotation(s->entity, atan2f(dy, dx));
                if (dist < 280.0f && s->state_timer <= 0.0f) {
                    s->phase_step = 1u;
                    s->state_timer = 0.42f;
                    s->dash_vec = fge_vec2(dir_x, dir_y);
                    fge_set_velocity(s->entity, 0.0f, 0.0f);
                }
            } else if (s->phase_step == 1u) {
                /* Telegraph flash before dash */
                fge_set_color(s->entity, fge_rgba(255, 245, 160, 255));
                fge_set_rotation(s->entity, atan2f(s->dash_vec.y, s->dash_vec.x));
                if (s->state_timer <= 0.0f) {
                    s->phase_step = 2u;
                    s->state_timer = 0.36f;
                    float dash_spd = 540.0f * mult;
                    fge_set_velocity(s->entity, s->dash_vec.x * dash_spd, s->dash_vec.y * dash_spd);
                }
            } else {
                if (s->state_timer <= 0.0f) {
                    s->phase_step = 0u;
                    s->state_timer = 1.3f;
                }
            }
        } else if (s->kind == ENEMY_SHOOTER) {
            float vx = 0.0f;
            float vy = 0.0f;
            if (dist > 340.0f) {
                vx = dir_x * s->speed;
                vy = dir_y * s->speed;
            } else if (dist < 220.0f) {
                vx = -dir_x * s->speed;
                vy = -dir_y * s->speed;
            } else {
                float strafe_sign = (i % 2u == 0u) ? 1.0f : -1.0f;
                vx = -dir_y * s->speed * 0.85f * strafe_sign;
                vy = dir_x * s->speed * 0.85f * strafe_sign;
            }
            fge_set_velocity(s->entity, vx, vy);

            s->fire_timer -= dt * mult;
            if (s->fire_timer <= 0.0f) {
                if (s->phase_step % 2u == 0u) {
                    emit_aimed_spread(ep.x, ep.y, ppos, 3u, 0.22f, 250.0f * mult, 10.0f, fge_rgba(255, 185, 75, 255));
                } else {
                    emit_radial_ring(ep.x, ep.y, 8u, g_game.elapsed + (float)i, 200.0f * mult, 9.5f, fge_rgba(240, 105, 255, 255));
                }
                s->phase_step += 1u;
                s->fire_timer = 1.65f;
            }
        } else if (s->kind == ENEMY_BOSS) {
            boss_found = true;
            g_game.boss_active = true;
            g_game.boss_hp = fge_get_hp(s->entity);
            g_game.boss_max_hp = s->max_hp;

            /* Smooth menacing orbit toward mid-range of player */
            float vx = 0.0f;
            float vy = 0.0f;
            if (dist > 290.0f) {
                vx = dir_x * s->speed;
                vy = dir_y * s->speed;
            } else if (dist < 200.0f) {
                vx = -dir_x * s->speed * 0.7f;
                vy = -dir_y * s->speed * 0.7f;
            }
            vx += -dir_y * s->speed * 0.65f;
            vy += dir_x * s->speed * 0.65f;
            fge_set_velocity(s->entity, vx, vy);
            fge_set_rotation(s->entity, g_boss_orbit_angle * 0.5f);

            /* Update Boss Multi-Part eye & orbiting armor blocks */
            if (fge_entity_is_alive(g_boss_eye)) {
                float pulse = 22.0f + 7.0f * sinf(g_game.elapsed * 6.0f);
                fge_set_position(g_boss_eye, ep.x, ep.y);
                fge_set_size(g_boss_eye, pulse, pulse);
                fge_set_rotation(g_boss_eye, -g_boss_orbit_angle);
            }
            for (uint32_t k = 0; k < 4u; ++k) {
                if (fge_entity_is_alive(g_boss_armor[k])) {
                    float a = g_boss_orbit_angle + (NEON_PI * 0.5f) * (float)k;
                    float ax = ep.x + cosf(a) * 58.0f;
                    float ay = ep.y + sinf(a) * 58.0f;
                    fge_set_position(g_boss_armor[k], ax, ay);
                    fge_set_rotation(g_boss_armor[k], a + NEON_PI * 0.25f);
                }
            }

            /* Boss Danmaku Curtain Patterns */
            s->fire_timer -= dt * mult;
            s->state_timer -= dt * mult;
            if (s->fire_timer <= 0.0f) {
                g_boss_spiral_angle += 0.28f;
                uint32_t arms = 6u;
                for (uint32_t a_i = 0; a_i < arms; ++a_i) {
                    float ang1 = g_boss_spiral_angle + (NEON_TAU * (float)a_i) / (float)arms;
                    float ang2 = -g_boss_spiral_angle * 0.85f + (NEON_TAU * (float)a_i) / (float)arms;
                    float bspd = 230.0f * mult;
                    spawn_enemy_bullet(ep.x, ep.y, cosf(ang1) * bspd, sinf(ang1) * bspd, 11.0f, fge_rgba(255, 85, 195, 255));
                    if (s->phase_step % 2u == 0u) {
                        spawn_enemy_bullet(ep.x, ep.y, cosf(ang2) * (bspd * 0.82f), sinf(ang2) * (bspd * 0.82f), 9.5f, fge_rgba(145, 110, 255, 255));
                    }
                }
                s->phase_step += 1u;
                s->fire_timer = 0.145f;
            }
            if (s->state_timer <= 0.0f) {
                emit_aimed_spread(ep.x, ep.y, ppos, 7u, 0.16f, 295.0f * mult, 12.0f, fge_rgba(255, 210, 80, 255));
                emit_radial_ring(ep.x, ep.y, 16u, g_game.elapsed * 0.7f, 185.0f * mult, 10.0f, fge_rgba(255, 95, 145, 255));
                s->state_timer = 1.45f;
            }
        }
    }

    if (!boss_found && g_game.boss_active) {
        hide_boss_parts();
    }

    /* Detect Enemy Bullets vs Player */
    uint32_t ebullet_hits = fge_detect_collisions(LAYER_ENEMY_BULLET, LAYER_PLAYER);
    for (uint32_t i = 0; i < ebullet_hits; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.a) || !fge_entity_is_alive(pair.b)) {
            continue;
        }
        FgeVec2 bp = fge_get_position(pair.a);
        fge_entity_despawn(pair.a);
        player_apply_damage(10.0f, bp);
    }

    /* Detect Enemy Bodies vs Player */
    uint32_t body_hits = fge_detect_collisions(LAYER_ENEMY, LAYER_PLAYER);
    for (uint32_t i = 0; i < body_hits; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.a) || !fge_entity_is_alive(pair.b)) {
            continue;
        }
        FgeVec2 ep = fge_get_position(pair.a);
        player_apply_damage(14.0f, ep);
    }

    /* Detect Player Bullets / Missiles / Blades vs Enemies (last so headless telemetry captures combat hits) */
    uint32_t hit_n = fge_detect_collisions(LAYER_PLAYER_BULLET, LAYER_ENEMY);
    for (uint32_t i = 0; i < hit_n; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.a) || !fge_entity_is_alive(pair.b)) {
            continue;
        }
        uint32_t btag = fge_get_tag(pair.a);
        float dmg = fge_get_hp(pair.a);
        if (dmg <= 0.0f) {
            dmg = 10.0f;
        }

        FgeVec2 hit_pos = fge_get_position(pair.a);
        if (btag != TAG_PLAYER_BLADE) {
            fge_entity_despawn(pair.a);
        }

        float ehp = fge_get_hp(pair.b) - dmg;
        fge_set_hp(pair.b, ehp);

        int32_t slot_idx = find_enemy_slot_by_entity(pair.b);
        if (slot_idx >= 0) {
            g_enemies[slot_idx].hit_flash = 0.07f;
            if (ehp <= 0.0f) {
                kill_enemy_slot(&g_enemies[slot_idx]);
            } else {
                fx_spawn_hit_spark(hit_pos.x, hit_pos.y, fge_rgba(160, 245, 255, 255));
            }
        }
    }

    /* Prune off-arena or expired enemy bullets */
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        EnemyBulletSlot *bs = &g_enemy_bullets[i];
        if (!fge_entity_is_alive(bs->entity)) {
            bs->entity = fge_entity_null();
            continue;
        }
        bs->age += dt;
        FgeVec2 bp = fge_get_position(bs->entity);
        if (bs->age >= bs->max_life ||
            bp.x < -64.0f || bp.x > NEON_ARENA_SIZE + 64.0f ||
            bp.y < -64.0f || bp.y > NEON_ARENA_SIZE + 64.0f) {
            fge_entity_despawn(bs->entity);
            bs->entity = fge_entity_null();
        }
    }

    /* Spawn next wave when cleared or reinforce periodically */
    uint32_t alive = enemies_alive_count();
    g_wave_spawn_timer += dt;
    if (alive == 0u || (alive < 4u && g_wave_spawn_timer > 4.5f)) {
        g_wave_spawn_timer = 0.0f;
        spawn_wave(g_game.wave_index + 1u);
    }
}

void enemies_shutdown(void) {
    for (uint32_t i = 0; i < MAX_ENEMIES; ++i) {
        if (fge_entity_is_alive(g_enemies[i].entity)) {
            fge_entity_despawn(g_enemies[i].entity);
        }
        g_enemies[i].entity = fge_entity_null();
        g_enemies[i].active = false;
    }
    for (uint32_t i = 0; i < MAX_ENEMY_BULLETS; ++i) {
        if (fge_entity_is_alive(g_enemy_bullets[i].entity)) {
            fge_entity_despawn(g_enemy_bullets[i].entity);
        }
        g_enemy_bullets[i].entity = fge_entity_null();
    }
    if (fge_entity_is_alive(g_boss_eye)) {
        fge_entity_despawn(g_boss_eye);
    }
    g_boss_eye = fge_entity_null();
    for (uint32_t k = 0; k < 4u; ++k) {
        if (fge_entity_is_alive(g_boss_armor[k])) {
            fge_entity_despawn(g_boss_armor[k]);
        }
        g_boss_armor[k] = fge_entity_null();
    }
}
