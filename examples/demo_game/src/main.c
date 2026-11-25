#include "game_shared.h"

#define STAR_COUNT 24
#define MAX_AI_BONUSES 64
#define TAG_AI_BONUS 4u
#define AI_EVENT_SPAWN_BONUS 1u
#define METRIC_ENTITY_COUNT 1u
#define METRIC_SCORE 2u

static FgeEntity g_stars[STAR_COUNT];
static FgeEntity g_ai_bonuses[MAX_AI_BONUSES];
static uint32_t g_ai_bonus_cursor = 0u;
static uint32_t g_score = 0u;

void game_add_score(uint32_t points) {
    g_score += points;
}

uint32_t game_get_score(void) {
    return g_score;
}

void fge_init(void) {
    g_score = 0u;
    g_ai_bonus_cursor = 0u;
    for (uint32_t i = 0; i < MAX_AI_BONUSES; ++i) {
        g_ai_bonuses[i] = fge_entity_null();
    }

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

    uint32_t ev_n = fge_poll_ai_events();
    for (uint32_t i = 0; i < ev_n; ++i) {
        FgeAiEvent ev = fge_get_ai_event(i);
        if (ev.kind == AI_EVENT_SPAWN_BONUS) {
            FgeEntity bonus = fge_spawn_sprite(
                ev.arg0,
                ev.arg1,
                20.0f,
                20.0f,
                fge_rgba(255, 220, 40, 255)
            );
            fge_set_tag(bonus, TAG_AI_BONUS);
            fge_set_hp(bonus, 1.0f);
            fge_set_z_order(bonus, 6.0f);
            fge_set_angular_velocity(bonus, 1.5f);

            uint32_t slot = g_ai_bonus_cursor % MAX_AI_BONUSES;
            if (fge_entity_is_alive(g_ai_bonuses[slot])) {
                fge_entity_despawn(g_ai_bonuses[slot]);
            }
            g_ai_bonuses[slot] = bonus;
            g_ai_bonus_cursor += 1u;
        }
    }

    fge_log_metric(METRIC_ENTITY_COUNT, (float)fge_entity_count());
    fge_log_metric(METRIC_SCORE, (float)g_score);
}

void fge_shutdown(void) {
    player_shutdown();
    enemy_shutdown();

    for (uint32_t i = 0; i < MAX_AI_BONUSES; ++i) {
        if (fge_entity_is_alive(g_ai_bonuses[i])) {
            fge_entity_despawn(g_ai_bonuses[i]);
        }
        g_ai_bonuses[i] = fge_entity_null();
    }

    for (uint32_t i = 0; i < STAR_COUNT; ++i) {
        if (fge_entity_is_alive(g_stars[i])) {
            fge_entity_despawn(g_stars[i]);
        }
        g_stars[i] = fge_entity_null();
    }
}
