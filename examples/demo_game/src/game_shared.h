#ifndef GAME_SHARED_H
#define GAME_SHARED_H

#include <stdbool.h>
#include <stdint.h>

#include "fge.h"

#define GAME_WIDTH  1280.0f
#define GAME_HEIGHT 720.0f
#define GAME_PI     3.14159265358979323846f

/* Collision layer bitmasks */
enum {
    LAYER_PLAYER        = 1u << 0,
    LAYER_ENEMY         = 1u << 1,
    LAYER_PLAYER_BULLET = 1u << 2,
    LAYER_ENEMY_BULLET  = 1u << 3
};

/* Gameplay entity tags */
enum {
    TAG_PLAYER = 1u,
    TAG_ENEMY  = 2u,
    TAG_BULLET = 3u
};

/* main.c */
void game_add_score(uint32_t points);
uint32_t game_get_score(void);

/* player.c */
void player_init(void);
void player_update(float dt);
void player_shutdown(void);
FgeEntity player_get_entity(void);
FgeVec2 player_get_position(void);

/* enemy.c */
void enemy_init(void);
void enemy_update(float dt);
void enemy_shutdown(void);
void enemy_spawn_wave(uint32_t wave_index);
uint32_t enemy_alive_count(void);

#endif /* GAME_SHARED_H */
