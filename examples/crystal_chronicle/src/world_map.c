#include "rpg_shared.h"

#include <math.h>

#define MAX_MAP_ENTITIES 420u
#define MAX_OBSTACLES    96u
#define MAX_LANTERNS     14u
#define MAX_WATER_GLINTS 16u

typedef struct MapObstacle {
    float x;
    float y;
    float radius;
} MapObstacle;

typedef struct LanternFx {
    FgeEntity glow;
    float base_size;
    float phase;
} LanternFx;

typedef struct WaterGlint {
    FgeEntity sprite;
    float base_x;
    float base_y;
    float phase;
} WaterGlint;

static FgeEntity g_map_entities[MAX_MAP_ENTITIES];
static uint32_t g_map_entity_count = 0u;

static MapObstacle g_obstacles[MAX_OBSTACLES];
static uint32_t g_obstacle_count = 0u;

static LanternFx g_lanterns[MAX_LANTERNS];
static uint32_t g_lantern_count = 0u;

static WaterGlint g_water_glints[MAX_WATER_GLINTS];
static uint32_t g_water_glint_count = 0u;

static FgeEntity g_crystal_spire = { 0xFFFFFFFFu, 0u };
static FgeEntity g_crystal_aura = { 0xFFFFFFFFu, 0u };
static FgeEntity g_altar_rune_inner = { 0xFFFFFFFFu, 0u };
static float g_map_anim_time = 0.0f;

static FgeEntity push_map_rect(
    float x,
    float y,
    float w,
    float h,
    float rot,
    float z,
    uint32_t flags,
    FgeColor col
) {
    if (g_map_entity_count >= MAX_MAP_ENTITIES) {
        return fge_entity_null();
    }
    FgeEntity e = fge_spawn_sprite(x, y, w, h, col);
    fge_set_rotation(e, rot);
    fge_set_z_order(e, z);
    fge_set_tag(e, (z < 0.0f) ? TAG_WORLD_GROUND : TAG_WORLD_PROP);
    fge_set_flags(e, flags);
    g_map_entities[g_map_entity_count++] = e;
    return e;
}

static void add_obstacle(float x, float y, float radius) {
    if (g_obstacle_count >= MAX_OBSTACLES) {
        return;
    }
    g_obstacles[g_obstacle_count].x = x;
    g_obstacles[g_obstacle_count].y = y;
    g_obstacles[g_obstacle_count].radius = radius;
    g_obstacle_count++;
}

static void add_lantern(float x, float y, float phase) {
    push_map_rect(x, y + 6.0f, 12.0f, 6.0f, 0.0f, -1.0f, FGE_FLAG_VISIBLE, fge_rgba(12, 16, 20, 95));
    push_map_rect(x, y - 6.0f, 6.0f, 22.0f, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(68, 52, 40, 255));
    push_map_rect(x, y - 18.0f, 10.0f, 10.0f, 0.0f, 1.1f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(255, 220, 120, 255));
    if (g_lantern_count < MAX_LANTERNS) {
        g_lanterns[g_lantern_count].glow = push_map_rect(
            x, y - 18.0f, 34.0f, 34.0f, 0.0f, -1.5f, FGE_FLAG_VISIBLE, fge_rgba(255, 195, 85, 65)
        );
        g_lanterns[g_lantern_count].base_size = 34.0f;
        g_lanterns[g_lantern_count].phase = phase;
        g_lantern_count++;
    }
}

static void add_pine_tree(float x, float y, float scale) {
    float tw = 16.0f * scale;
    float th = 22.0f * scale;
    push_map_rect(x, y + 8.0f * scale, 48.0f * scale, 18.0f * scale, 0.0f, -1.0f, FGE_FLAG_VISIBLE, fge_rgba(10, 22, 14, 95));
    push_map_rect(x, y, tw, th, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(92, 62, 40, 255));
    /* Layered emerald canopies in layer 1 so Hero walks cleanly behind or in front */
    push_map_rect(
        x, y - 22.0f * scale, 56.0f * scale, 34.0f * scale,
        0.0f, 1.15f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(28, 92, 52, 255)
    );
    push_map_rect(
        x, y - 38.0f * scale, 42.0f * scale, 26.0f * scale,
        0.0f, 1.35f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(40, 118, 66, 255)
    );
    push_map_rect(
        x, y - 50.0f * scale, 24.0f * scale, 18.0f * scale,
        0.0f, 1.50f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(58, 145, 82, 255)
    );
    add_obstacle(x, y + 2.0f, 18.0f * scale);
}

static void add_village_house(float x, float y, FgeColor roof_col) {
    /* Soft ground shadow */
    push_map_rect(x, y + 14.0f, 154.0f, 76.0f, 0.0f, -1.0f, FGE_FLAG_VISIBLE, fge_rgba(16, 22, 18, 85));
    /* Timber & plaster wall base (layer 0) */
    push_map_rect(x, y - 8.0f, 136.0f, 58.0f, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(226, 210, 180, 255));
    /* Wooden door & warm windows on front face (y > wall center so drawn on top) */
    push_map_rect(x, y + 7.0f, 26.0f, 28.0f, 0.0f, 0.35f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(105, 68, 42, 255));
    push_map_rect(x - 40.0f, y + 2.0f, 22.0f, 18.0f, 0.0f, 0.35f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(255, 215, 110, 255));
    push_map_rect(x + 40.0f, y + 2.0f, 22.0f, 18.0f, 0.0f, 0.35f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(255, 215, 110, 255));
    /* Pitched roof (layer 1) */
    push_map_rect(x, y - 42.0f, 152.0f, 44.0f, 0.0f, 1.25f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, roof_col);
    push_map_rect(x, y - 56.0f, 126.0f, 16.0f, 0.0f, 1.40f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(245, 228, 195, 140));
    add_obstacle(x - 34.0f, y - 4.0f, 38.0f);
    add_obstacle(x + 34.0f, y - 4.0f, 38.0f);
}

static void add_ruins_pillar(float x, float y) {
    push_map_rect(x, y + 10.0f, 42.0f, 22.0f, 0.0f, -1.0f, FGE_FLAG_VISIBLE, fge_rgba(10, 10, 18, 110));
    push_map_rect(x, y - 10.0f, 34.0f, 52.0f, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(82, 78, 98, 255));
    push_map_rect(x, y - 34.0f, 40.0f, 14.0f, 0.0f, 1.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(108, 102, 128, 255));
    if (g_lantern_count < MAX_LANTERNS) {
        g_lanterns[g_lantern_count].glow = push_map_rect(
            x, y - 42.0f, 24.0f, 24.0f, RPG_PI * 0.25f, 1.45f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(185, 105, 255, 220)
        );
        g_lanterns[g_lantern_count].base_size = 24.0f;
        g_lanterns[g_lantern_count].phase = x * 0.01f;
        g_lantern_count++;
    }
    add_obstacle(x, y + 2.0f, 22.0f);
}

void world_map_init(void) {
    g_map_entity_count = 0u;
    g_obstacle_count = 0u;
    g_lantern_count = 0u;
    g_water_glint_count = 0u;
    g_map_anim_time = 0.0f;
    for (uint32_t i = 0; i < MAX_MAP_ENTITIES; ++i) {
        g_map_entities[i] = fge_entity_null();
    }

    /* 1. Zone Ground Biomes (z = -10..-6) */
    /* Northern Ancient Ruins (y: 0..560) */
    push_map_rect(RPG_WORLD_W * 0.5f, 280.0f, RPG_WORLD_W, 560.0f, 0.0f, -10.0f, FGE_FLAG_VISIBLE, fge_rgba(36, 34, 48, 255));
    /* Central Whispering Woods (y: 560..1420) */
    push_map_rect(RPG_WORLD_W * 0.5f, 990.0f, RPG_WORLD_W, 860.0f, 0.0f, -10.0f, FGE_FLAG_VISIBLE, fge_rgba(32, 74, 46, 255));
    /* Peaceful Southern Village (y: 1420..2000) */
    push_map_rect(RPG_WORLD_W * 0.5f, 1710.0f, RPG_WORLD_W, 580.0f, 0.0f, -10.0f, FGE_FLAG_VISIBLE, fge_rgba(62, 114, 58, 255));

    /* 2. Cobblestone Road & Village Plaza (z = -8..-6) */
    push_map_rect(1100.0f, 1650.0f, 440.0f, 280.0f, 0.0f, -8.0f, FGE_FLAG_VISIBLE, fge_rgba(148, 138, 122, 255));
    push_map_rect(1100.0f, 1650.0f, 412.0f, 252.0f, 0.0f, -7.8f, FGE_FLAG_VISIBLE, fge_rgba(164, 152, 134, 255));
    /* Main North-South Adventure Road */
    push_map_rect(1100.0f, 1265.0f, 118.0f, 490.0f, 0.0f, -7.9f, FGE_FLAG_VISIBLE, fge_rgba(146, 132, 108, 255));
    push_map_rect(1100.0f, 755.0f, 118.0f, 510.0f, 0.0f, -7.9f, FGE_FLAG_VISIBLE, fge_rgba(128, 116, 98, 255));

    /* Subtle plaza stone accents */
    for (int i = -2; i <= 2; ++i) {
        for (int j = -1; j <= 1; ++j) {
            float sx = 1100.0f + (float)i * 78.0f;
            float sy = 1650.0f + (float)j * 68.0f;
            push_map_rect(sx, sy, 52.0f, 38.0f, 0.0f, -7.4f, FGE_FLAG_VISIBLE, fge_rgba(176, 165, 146, 190));
        }
    }

    /* 3. Sparkling River & Wooden Bridge (y: 1010..1075) */
    push_map_rect(RPG_WORLD_W * 0.5f, 1042.5f, RPG_WORLD_W, 74.0f, 0.0f, -7.0f, FGE_FLAG_VISIBLE, fge_rgba(28, 92, 148, 255));
    push_map_rect(RPG_WORLD_W * 0.5f, 1042.5f, RPG_WORLD_W, 54.0f, 0.0f, -6.8f, FGE_FLAG_VISIBLE, fge_rgba(44, 132, 196, 255));
    for (uint32_t i = 0; i < MAX_WATER_GLINTS; ++i) {
        float wx = 120.0f + (float)i * 130.0f;
        if (wx > RPG_BRIDGE_MIN_X - 20.0f && wx < RPG_BRIDGE_MAX_X + 20.0f) {
            wx += 160.0f;
        }
        float wy = 1034.0f + ((i % 2u == 0u) ? -6.0f : 10.0f);
        g_water_glints[g_water_glint_count].sprite = push_map_rect(
            wx, wy, 36.0f, 6.0f, 0.0f, -6.5f, FGE_FLAG_VISIBLE, fge_rgba(155, 225, 255, 165)
        );
        g_water_glints[g_water_glint_count].base_x = wx;
        g_water_glints[g_water_glint_count].base_y = wy;
        g_water_glints[g_water_glint_count].phase = (float)i * 0.7f;
        g_water_glint_count++;
    }

    /* Wooden Bridge across the river */
    push_map_rect(1100.0f, 1042.5f, 148.0f, 98.0f, 0.0f, -5.5f, FGE_FLAG_VISIBLE, fge_rgba(116, 76, 44, 255));
    for (int p = -3; p <= 3; ++p) {
        push_map_rect(1100.0f, 1042.5f + (float)p * 13.0f, 140.0f, 9.0f, 0.0f, -5.2f, FGE_FLAG_VISIBLE, fge_rgba(148, 102, 62, 255));
    }
    push_map_rect(1030.0f, 1042.5f, 10.0f, 102.0f, 0.0f, 0.25f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(88, 56, 32, 255));
    push_map_rect(1170.0f, 1042.5f, 10.0f, 102.0f, 0.0f, 0.25f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(88, 56, 32, 255));

    /* 4. Northern Ancient Ruins Boss Arena & Rune Altar */
    push_map_rect(1100.0f, 300.0f, 920.0f, 410.0f, 0.0f, -8.0f, FGE_FLAG_VISIBLE, fge_rgba(58, 54, 76, 255));
    push_map_rect(1100.0f, 300.0f, 880.0f, 370.0f, 0.0f, -7.6f, FGE_FLAG_VISIBLE, fge_rgba(72, 68, 92, 255));
    /* Glowing Rune Altar */
    push_map_rect(1100.0f, 290.0f, 190.0f, 190.0f, RPG_PI * 0.25f, -7.0f, FGE_FLAG_VISIBLE, fge_rgba(135, 65, 205, 110));
    g_altar_rune_inner = push_map_rect(1100.0f, 290.0f, 130.0f, 130.0f, 0.0f, -6.8f, FGE_FLAG_VISIBLE, fge_rgba(195, 115, 255, 155));

    /* Ruins Boundary Walls (leaving wide southern entrance x=1010..1190) */
    push_map_rect(825.0f, 515.0f, 370.0f, 30.0f, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(68, 64, 84, 255));
    push_map_rect(1375.0f, 515.0f, 370.0f, 30.0f, 0.0f, 0.2f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(68, 64, 84, 255));
    add_obstacle(760.0f, 515.0f, 26.0f);
    add_obstacle(920.0f, 515.0f, 26.0f);
    add_obstacle(1280.0f, 515.0f, 26.0f);
    add_obstacle(1440.0f, 515.0f, 26.0f);

    /* Ruins Pillars */
    add_ruins_pillar(780.0f, 210.0f);
    add_ruins_pillar(1420.0f, 210.0f);
    add_ruins_pillar(780.0f, 400.0f);
    add_ruins_pillar(1420.0f, 400.0f);
    add_ruins_pillar(980.0f, 515.0f);
    add_ruins_pillar(1220.0f, 515.0f);

    /* 5. Village Houses, Sanctuary Crystal & Lanterns */
    add_village_house(760.0f, 1545.0f, fge_rgba(182, 68, 58, 255));
    add_village_house(1440.0f, 1545.0f, fge_rgba(62, 108, 174, 255));
    add_village_house(775.0f, 1795.0f, fge_rgba(186, 128, 52, 255));
    add_village_house(1425.0f, 1795.0f, fge_rgba(154, 74, 122, 255));

    /* Glowing Sanctuary Crystal in Village Square */
    push_map_rect(RPG_CRYSTAL_X, RPG_CRYSTAL_Y + 12.0f, 46.0f, 22.0f, 0.0f, -1.0f, FGE_FLAG_VISIBLE, fge_rgba(10, 28, 36, 110));
    push_map_rect(RPG_CRYSTAL_X, RPG_CRYSTAL_Y + 4.0f, 38.0f, 18.0f, 0.0f, 0.15f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(165, 172, 188, 255));
    g_crystal_aura = push_map_rect(RPG_CRYSTAL_X, RPG_CRYSTAL_Y - 16.0f, 56.0f, 56.0f, RPG_PI * 0.25f, -5.0f, FGE_FLAG_VISIBLE, fge_rgba(75, 225, 255, 85));
    g_crystal_spire = push_map_rect(RPG_CRYSTAL_X, RPG_CRYSTAL_Y - 18.0f, 24.0f, 34.0f, RPG_PI * 0.25f, 1.25f, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT, fge_rgba(95, 238, 255, 245));
    add_obstacle(RPG_CRYSTAL_X, RPG_CRYSTAL_Y + 2.0f, 22.0f);

    /* Village & Road Lanterns */
    add_lantern(955.0f, 1665.0f, 0.0f);
    add_lantern(1245.0f, 1665.0f, 1.1f);
    add_lantern(1015.0f, 1455.0f, 2.2f);
    add_lantern(1185.0f, 1455.0f, 3.0f);
    add_lantern(1015.0f, 1130.0f, 4.1f);
    add_lantern(1185.0f, 955.0f, 5.2f);

    /* 6. Whispering Woods Pine Trees (carefully placed clear of main path & combat clearings) */
    static const float tree_coords[][3] = {
        { 560.0f, 1340.0f, 1.1f }, { 710.0f, 1290.0f, 1.0f }, { 850.0f, 1365.0f, 1.2f },
        { 1330.0f, 1360.0f, 1.15f }, { 1490.0f, 1295.0f, 1.0f }, { 1640.0f, 1345.0f, 1.1f },
        { 620.0f, 1165.0f, 1.1f }, { 795.0f, 1145.0f, 0.95f }, { 1410.0f, 1150.0f, 1.05f },
        { 1590.0f, 1170.0f, 1.15f }, { 560.0f, 920.0f, 1.2f }, { 735.0f, 895.0f, 1.0f },
        { 890.0f, 935.0f, 0.95f }, { 1320.0f, 930.0f, 1.0f }, { 1485.0f, 890.0f, 1.15f },
        { 1650.0f, 920.0f, 1.1f }, { 640.0f, 725.0f, 1.1f }, { 810.0f, 700.0f, 1.0f },
        { 1390.0f, 705.0f, 1.05f }, { 1565.0f, 730.0f, 1.15f },
        { 520.0f, 1610.0f, 1.1f }, { 1675.0f, 1615.0f, 1.1f },
        { 560.0f, 1840.0f, 1.05f }, { 1630.0f, 1845.0f, 1.05f }
    };
    uint32_t tree_count = (uint32_t)(sizeof(tree_coords) / sizeof(tree_coords[0]));
    for (uint32_t i = 0; i < tree_count; ++i) {
        add_pine_tree(tree_coords[i][0], tree_coords[i][1], tree_coords[i][2]);
    }

    /* 7. Wildflowers scattered across Village & Woods (z = -5.0) */
    for (uint32_t i = 0; i < 34u; ++i) {
        float fx = 520.0f + (float)((i * 197u) % 1160u);
        float fy = 640.0f + (float)((i * 263u) % 1200u);
        if (fy > RPG_RIVER_MIN_Y - 25.0f && fy < RPG_RIVER_MAX_Y + 25.0f) {
            continue;
        }
        if (fx > 1030.0f && fx < 1170.0f) {
            continue;
        }
        FgeColor fcol = (i % 3u == 0u) ? fge_rgba(255, 228, 95, 235)
                      : (i % 3u == 1u) ? fge_rgba(255, 148, 195, 235)
                                       : fge_rgba(145, 225, 255, 235);
        push_map_rect(fx, fy, 8.0f, 8.0f, 0.4f, -5.0f, FGE_FLAG_VISIBLE, fcol);
    }
}

void world_map_update(float dt) {
    g_map_anim_time += dt;

    /* Sanctuary crystal glow & hover */
    if (fge_entity_is_alive(g_crystal_spire)) {
        float bob = sinf(g_map_anim_time * 2.6f) * 4.0f;
        fge_set_position(g_crystal_spire, RPG_CRYSTAL_X, RPG_CRYSTAL_Y - 18.0f + bob);
        fge_set_rotation(g_crystal_spire, RPG_PI * 0.25f + sinf(g_map_anim_time * 1.4f) * 0.12f);
    }
    if (fge_entity_is_alive(g_crystal_aura)) {
        float pulse = 54.0f + sinf(g_map_anim_time * 3.2f) * 10.0f;
        fge_set_size(g_crystal_aura, pulse, pulse);
        fge_set_rotation(g_crystal_aura, g_map_anim_time * 0.65f);
    }
    if (fge_entity_is_alive(g_altar_rune_inner)) {
        fge_set_rotation(g_altar_rune_inner, -g_map_anim_time * 0.4f);
    }

    /* Lantern & braziers flicker */
    for (uint32_t i = 0; i < g_lantern_count; ++i) {
        if (fge_entity_is_alive(g_lanterns[i].glow)) {
            float sz = g_lanterns[i].base_size + sinf(g_map_anim_time * 5.0f + g_lanterns[i].phase) * 4.0f;
            fge_set_size(g_lanterns[i].glow, sz, sz);
        }
    }

    /* River shimmer */
    for (uint32_t i = 0; i < g_water_glint_count; ++i) {
        if (fge_entity_is_alive(g_water_glints[i].sprite)) {
            float sx = g_water_glints[i].base_x + sinf(g_map_anim_time * 1.6f + g_water_glints[i].phase) * 16.0f;
            fge_set_position(g_water_glints[i].sprite, sx, g_water_glints[i].base_y);
        }
    }
}

void world_map_resolve_movement(float *x, float *y, float radius) {
    /* World perimeter bounds */
    *x = rpg_clamp_f32(*x, RPG_MARGIN + radius, RPG_WORLD_W - RPG_MARGIN - radius);
    *y = rpg_clamp_f32(*y, RPG_MARGIN + radius, RPG_WORLD_H - RPG_MARGIN - radius);

    /* River barrier outside the wooden bridge */
    bool on_bridge_x = (*x >= RPG_BRIDGE_MIN_X + radius * 0.4f && *x <= RPG_BRIDGE_MAX_X - radius * 0.4f);
    if (!on_bridge_x) {
        float river_top = RPG_RIVER_MIN_Y - radius;
        float river_bot = RPG_RIVER_MAX_Y + radius;
        if (*y > river_top && *y < river_bot) {
            float mid = (river_top + river_bot) * 0.5f;
            *y = (*y < mid) ? river_top : river_bot;
        }
    }

    /* Circular tree/house/pillar obstacles */
    for (uint32_t i = 0; i < g_obstacle_count; ++i) {
        float dx = *x - g_obstacles[i].x;
        float dy = *y - g_obstacles[i].y;
        float min_d = radius + g_obstacles[i].radius;
        float d2 = dx * dx + dy * dy;
        if (d2 < min_d * min_d) {
            float d = sqrtf(d2);
            if (d > 0.001f) {
                float push = min_d - d;
                *x += (dx / d) * push;
                *y += (dy / d) * push;
            } else {
                *y += min_d;
            }
        }
    }
}

void world_map_shutdown(void) {
    for (uint32_t i = 0; i < MAX_MAP_ENTITIES; ++i) {
        if (fge_entity_is_alive(g_map_entities[i])) {
            fge_entity_despawn(g_map_entities[i]);
        }
        g_map_entities[i] = fge_entity_null();
    }
    g_map_entity_count = 0u;
    g_obstacle_count = 0u;
    g_lantern_count = 0u;
    g_water_glint_count = 0u;
    g_crystal_spire = fge_entity_null();
    g_crystal_aura = fge_entity_null();
    g_altar_rune_inner = fge_entity_null();
}
