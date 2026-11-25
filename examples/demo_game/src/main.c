#include "game_shared.h"

#define STAR_COUNT 24

static FgeEntity g_stars[STAR_COUNT];
static uint32_t g_score = 0u;

void game_add_score(uint32_t points) {
    g_score += points;
}

uint32_t game_get_score(void) {
    return g_score;
}

void fge_init(void) {
    g_score = 0u;
    fge_camera_set(GAME_WIDTH * 0.5f, GAME_HEIGHT * 0.5f, 1.0f);

    for (uint32_t i = 0; i < STAR_COUNT; ++i) {
        float x = fge_rand_range(16.0f, GAME_WIDTH - 16.0f);
        float y = fge_rand_range(0.0f, GAME_HEIGHT);
        float size = fge_rand_range(2.0f, 4.5f);
        float speed = 35.0f + size * 20.0f;
        uint8_t shade = (uint8_t)(80u + (i % 4u) * 35u);
        FgeEntity star = fge_spawn_sprite(x, y, size, size, fge_rgba(shade, shade, 220, 180));
        fge_set_velocity(star, 0.0f, speed);
        fge_set_z_order(star, -10.0f);
        g_stars[i] = star;
    }

    player_init();
    enemy_init();
}

void fge_update(float dt) {
    for (uint32_t i = 0; i < STAR_COUNT; ++i) {
        FgeEntity star = g_stars[i];
        if (!fge_entity_is_alive(star)) {
            continue;
        }
        FgeVec2 pos = fge_get_position(star);
        if (pos.y > GAME_HEIGHT + 8.0f) {
            fge_set_position(star, fge_rand_range(16.0f, GAME_WIDTH - 16.0f), -8.0f);
        }
    }

    player_update(dt);
    enemy_update(dt);
}

void fge_shutdown(void) {
    player_shutdown();
    enemy_shutdown();

    for (uint32_t i = 0; i < STAR_COUNT; ++i) {
        if (fge_entity_is_alive(g_stars[i])) {
            fge_entity_despawn(g_stars[i]);
        }
        g_stars[i] = fge_entity_null();
    }
}
