#include "game_shared.h"

#include <math.h>

#define MAX_PLAYER_BULLETS 128
#define PLAYER_SPEED       360.0f
#define PLAYER_FOCUS_SPEED 170.0f
#define PLAYER_FIRE_PERIOD 0.08f
#define PLAYER_BULLET_VEL  (-1200.0f)

static FgeEntity g_player;
static FgeEntity g_player_bullets[MAX_PLAYER_BULLETS];
static float g_fire_cooldown = 0.0f;
static float g_player_elapsed = 0.0f;

static void spawn_player_bullet(float x, float y, float vx, float vy) {
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        if (!fge_entity_is_alive(g_player_bullets[i])) {
            FgeEntity b = fge_spawn_sprite(x, y, 8.0f, 18.0f, fge_rgba(160, 245, 255, 255));
            fge_set_velocity(b, vx, vy);
            fge_set_collider(b, 6.0f, LAYER_PLAYER_BULLET);
            fge_set_tag(b, TAG_BULLET);
            fge_set_z_order(b, 5.0f);
            g_player_bullets[i] = b;
            return;
        }
    }
}

static void prune_offscreen_bullets(void) {
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        FgeEntity b = g_player_bullets[i];
        if (!fge_entity_is_alive(b)) {
            g_player_bullets[i] = fge_entity_null();
            continue;
        }
        FgeVec2 pos = fge_get_position(b);
        if (pos.y < -32.0f || pos.y > GAME_HEIGHT + 32.0f ||
            pos.x < -32.0f || pos.x > GAME_WIDTH + 32.0f) {
            fge_entity_despawn(b);
            g_player_bullets[i] = fge_entity_null();
        }
    }
}

void player_init(void) {
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        g_player_bullets[i] = fge_entity_null();
    }

    g_player = fge_spawn_sprite(640.0f, 560.0f, 28.0f, 28.0f, fge_rgba(80, 220, 255, 255));
    fge_set_hp(g_player, 100.0f);
    fge_set_collider(g_player, 10.0f, LAYER_PLAYER);
    fge_set_tag(g_player, TAG_PLAYER);
    fge_set_z_order(g_player, 10.0f);

    g_fire_cooldown = 0.0f;
    g_player_elapsed = 0.0f;
}

void player_update(float dt) {
    if (!fge_entity_is_alive(g_player)) {
        return;
    }

    g_player_elapsed += dt;
    if (g_fire_cooldown > 0.0f) {
        g_fire_cooldown -= dt;
    }

    float move_x = 0.0f;
    float move_y = 0.0f;
    bool has_input = false;

    if (fge_key_down(FGE_KEY_A) || fge_key_down(FGE_KEY_LEFT)) {
        move_x -= 1.0f;
        has_input = true;
    }
    if (fge_key_down(FGE_KEY_D) || fge_key_down(FGE_KEY_RIGHT)) {
        move_x += 1.0f;
        has_input = true;
    }
    if (fge_key_down(FGE_KEY_W) || fge_key_down(FGE_KEY_UP)) {
        move_y -= 1.0f;
        has_input = true;
    }
    if (fge_key_down(FGE_KEY_S) || fge_key_down(FGE_KEY_DOWN)) {
        move_y += 1.0f;
        has_input = true;
    }

    bool manual_fire = fge_key_down(FGE_KEY_SPACE) || fge_key_down(FGE_KEY_Z);
    if (manual_fire) {
        has_input = true;
    }

    float vx = 0.0f;
    float vy = 0.0f;
    bool wants_fire = manual_fire;

    if (has_input) {
        float len_sq = move_x * move_x + move_y * move_y;
        if (len_sq > 0.0f) {
            float inv_len = 1.0f / sqrtf(len_sq);
            float speed = (fge_key_down(FGE_KEY_LSHIFT) || fge_key_down(FGE_KEY_RSHIFT))
                              ? PLAYER_FOCUS_SPEED
                              : PLAYER_SPEED;
            vx = move_x * inv_len * speed;
            vy = move_y * inv_len * speed;
        }
    } else {
        /* Autonomous strafe/fire demo pattern when running headless or idle */
        vx = sinf(g_player_elapsed * 2.4f) * 180.0f;
        vy = cosf(g_player_elapsed * 1.6f) * 35.0f;
        wants_fire = true;
    }

    FgeVec2 pos = fge_get_position(g_player);
    float clamped_x = pos.x;
    float clamped_y = pos.y;
    if (clamped_x < 32.0f) {
        clamped_x = 32.0f;
        if (vx < 0.0f) {
            vx = 0.0f;
        }
    } else if (clamped_x > GAME_WIDTH - 32.0f) {
        clamped_x = GAME_WIDTH - 32.0f;
        if (vx > 0.0f) {
            vx = 0.0f;
        }
    }
    if (clamped_y < 120.0f) {
        clamped_y = 120.0f;
        if (vy < 0.0f) {
            vy = 0.0f;
        }
    } else if (clamped_y > GAME_HEIGHT - 32.0f) {
        clamped_y = GAME_HEIGHT - 32.0f;
        if (vy > 0.0f) {
            vy = 0.0f;
        }
    }
    if (clamped_x != pos.x || clamped_y != pos.y) {
        fge_set_position(g_player, clamped_x, clamped_y);
    }

    fge_set_velocity(g_player, vx, vy);

    if (wants_fire && g_fire_cooldown <= 0.0f) {
        spawn_player_bullet(clamped_x - 10.0f, clamped_y - 20.0f, -35.0f, PLAYER_BULLET_VEL);
        spawn_player_bullet(clamped_x, clamped_y - 22.0f, 0.0f, PLAYER_BULLET_VEL);
        spawn_player_bullet(clamped_x + 10.0f, clamped_y - 20.0f, 35.0f, PLAYER_BULLET_VEL);
        g_fire_cooldown = PLAYER_FIRE_PERIOD;
    }

    prune_offscreen_bullets();

    /* Process enemy bullets hitting the player */
    uint32_t hits = fge_detect_collisions(LAYER_ENEMY_BULLET, LAYER_PLAYER);
    for (uint32_t i = 0; i < hits; ++i) {
        FgeCollisionPair pair = fge_get_collision(i);
        if (!fge_entity_is_alive(pair.a) || !fge_entity_is_alive(pair.b)) {
            continue;
        }
        fge_entity_despawn(pair.a);
        float hp = fge_get_hp(pair.b) - 5.0f;
        if (hp <= 0.0f) {
            hp = 100.0f;
        }
        fge_set_hp(pair.b, hp);
    }
}

void player_shutdown(void) {
    for (uint32_t i = 0; i < MAX_PLAYER_BULLETS; ++i) {
        if (fge_entity_is_alive(g_player_bullets[i])) {
            fge_entity_despawn(g_player_bullets[i]);
        }
        g_player_bullets[i] = fge_entity_null();
    }
    if (fge_entity_is_alive(g_player)) {
        fge_entity_despawn(g_player);
    }
    g_player = fge_entity_null();
}

FgeEntity player_get_entity(void) {
    return g_player;
}

FgeVec2 player_get_position(void) {
    if (!fge_entity_is_alive(g_player)) {
        return fge_vec2(640.0f, 560.0f);
    }
    return fge_get_position(g_player);
}
