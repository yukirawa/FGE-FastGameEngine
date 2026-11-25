#include "neon_shared.h"

#include <math.h>

#define PLAYER_SPEED       350.0f
#define PLAYER_FOCUS_SPEED 195.0f
#define PLAYER_DASH_SPEED  880.0f
#define PLAYER_DASH_DUR    0.18f
#define PLAYER_DASH_CD     1.5f
#define PLAYER_NOVA_CD     8.0f
#define PLAYER_HIT_INVULN  0.75f

static FgeEntity g_hull;
static FgeEntity g_nose;
static FgeEntity g_wing_l;
static FgeEntity g_wing_r;
static FgeEntity g_focus_dot;
static FgeEntity g_core_ring[4];

static float g_idle_timer = NEON_AUTOPILOT_DELAY;
static float g_dash_timer = 0.0f;
static float g_dash_cooldown = 0.0f;
static float g_nova_cooldown = 0.0f;
static float g_thruster_timer = 0.0f;
static float g_ring_angle = 0.0f;
static float g_heading_angle = -NEON_PI * 0.5f;
static FgeVec2 g_dash_dir = { 1.0f, 0.0f };
static FgeVec2 g_last_mouse = { 0.0f, 0.0f };
static float g_mouse_active_timer = 0.0f;
static bool g_mouse_initialized = false;

static void update_ship_visuals(FgeVec2 pos, FgeVec2 vel, bool focus_mode) {
    float speed_sq = vel.x * vel.x + vel.y * vel.y;
    if (speed_sq > 100.0f) {
        g_heading_angle = atan2f(vel.y, vel.x);
    }

    float aim_angle = atan2f(g_game.player_aim_dir.y, g_game.player_aim_dir.x);
    float c_aim = cosf(aim_angle);
    float s_aim = sinf(aim_angle);
    float c_hd = cosf(g_heading_angle);
    float s_hd = sinf(g_heading_angle);

    /* Flash white on hit, blink subtly during i-frames, cyan normally */
    FgeColor hull_col = fge_rgba(65, 235, 255, 255);
    FgeColor wing_col = fge_rgba(35, 130, 255, 255);
    if (g_game.player_hit_flash > 0.0f) {
        hull_col = fge_rgba(255, 90, 120, 255);
        wing_col = fge_rgba(255, 180, 190, 255);
    } else if (g_dash_timer > 0.0f) {
        hull_col = fge_rgba(220, 255, 255, 255);
        wing_col = fge_rgba(110, 240, 255, 255);
    } else if (g_game.player_invuln > 0.0f) {
        uint8_t blink_a = ((uint32_t)(g_game.elapsed * 22.0f) % 2u == 0u) ? 255u : 120u;
        hull_col.a = blink_a;
        wing_col.a = blink_a;
    }

    if (fge_entity_is_alive(g_hull)) {
        fge_set_rotation(g_hull, aim_angle + NEON_PI * 0.25f);
        fge_set_color(g_hull, hull_col);
    }
    if (fge_entity_is_alive(g_nose)) {
        fge_set_position(g_nose, pos.x + c_aim * 13.0f, pos.y + s_aim * 13.0f);
        fge_set_rotation(g_nose, aim_angle);
        fge_set_color(g_nose, fge_rgba(235, 255, 255, hull_col.a));
    }
    if (fge_entity_is_alive(g_wing_l)) {
        float px = pos.x - c_hd * 5.0f - s_hd * 14.0f;
        float py = pos.y - s_hd * 5.0f + c_hd * 14.0f;
        fge_set_position(g_wing_l, px, py);
        fge_set_rotation(g_wing_l, g_heading_angle + 0.35f);
        fge_set_color(g_wing_l, wing_col);
    }
    if (fge_entity_is_alive(g_wing_r)) {
        float px = pos.x - c_hd * 5.0f + s_hd * 14.0f;
        float py = pos.y - s_hd * 5.0f - c_hd * 14.0f;
        fge_set_position(g_wing_r, px, py);
        fge_set_rotation(g_wing_r, g_heading_angle - 0.35f);
        fge_set_color(g_wing_r, wing_col);
    }

    /* Rotating 4-arc energy core ring */
    float ring_radius = focus_mode ? 18.0f : 23.0f;
    FgeColor ring_col = (g_game.player_level >= 5u)
                            ? fge_rgba(255, 215, 80, 220)
                            : fge_rgba(120, 245, 255, 200);
    if (g_game.autopilot_active) {
        ring_col = fge_rgba(120, 255, 195, 210);
    }
    for (uint32_t i = 0; i < 4u; ++i) {
        if (!fge_entity_is_alive(g_core_ring[i])) {
            continue;
        }
        float a = g_ring_angle + ((float)i * NEON_PI * 0.5f);
        fge_set_position(g_core_ring[i], pos.x + cosf(a) * ring_radius, pos.y + sinf(a) * ring_radius);
        fge_set_rotation(g_core_ring[i], a + NEON_PI * 0.5f);
        fge_set_color(g_core_ring[i], ring_col);
    }

    /* Core hitbox indicator dot */
    if (fge_entity_is_alive(g_focus_dot)) {
        fge_set_position(g_focus_dot, pos.x, pos.y);
        float pulse = 5.5f + 1.5f * sinf(g_game.elapsed * 8.0f);
        fge_set_size(g_focus_dot, focus_mode ? 8.0f : pulse, focus_mode ? 8.0f : pulse);
        fge_set_color(
            g_focus_dot,
            focus_mode ? fge_rgba(255, 255, 255, 255) : fge_rgba(255, 110, 215, 220)
        );
    }
}

static FgeVec2 compute_auto_aim(FgeVec2 pos) {
    uint32_t count = fge_query_circle(pos.x, pos.y, 950.0f, LAYER_ENEMY);
    float best_dist_sq = 1e18f;
    FgeVec2 best_dir = g_game.player_aim_dir;
    bool found = false;

    for (uint32_t i = 0; i < count; ++i) {
        FgeEntity e = fge_get_query_entity(i);
        if (!fge_entity_is_alive(e)) {
            continue;
        }
        FgeVec2 ep = fge_get_position(e);
        float dx = ep.x - pos.x;
        float dy = ep.y - pos.y;
        float d2 = dx * dx + dy * dy;
        if (d2 > 4.0f && d2 < best_dist_sq) {
            best_dist_sq = d2;
            float inv = 1.0f / sqrtf(d2);
            best_dir = fge_vec2(dx * inv, dy * inv);
            found = true;
        }
    }

    if (!found) {
        return g_game.player_aim_dir;
    }
    return best_dir;
}

static void compute_autopilot_move(FgeVec2 pos, float *out_mx, float *out_my, bool *out_dash, bool *out_nova) {
    float steer_x = 0.0f;
    float steer_y = 0.0f;
    float bullet_danger = 0.0f;

    /* 1. Weave around nearby enemy bullets */
    uint32_t b_count = fge_query_circle(pos.x, pos.y, 210.0f, LAYER_ENEMY_BULLET);
    float weave_sign = (sinf(g_game.elapsed * 0.45f) >= 0.0f) ? 1.0f : -1.0f;
    for (uint32_t i = 0; i < b_count; ++i) {
        FgeEntity b = fge_get_query_entity(i);
        if (!fge_entity_is_alive(b)) {
            continue;
        }
        FgeVec2 bp = fge_get_position(b);
        float dx = pos.x - bp.x;
        float dy = pos.y - bp.y;
        float d2 = dx * dx + dy * dy + 64.0f;
        float dist = sqrtf(d2);
        float w = (210.0f - dist) / 210.0f;
        if (w > 0.0f) {
            float inv = 1.0f / dist;
            float nx = dx * inv;
            float ny = dy * inv;
            /* Repel + perpendicular weave */
            steer_x += (nx * 2.2f - ny * 0.9f * weave_sign) * w * w * 4.5f;
            steer_y += (ny * 2.2f + nx * 0.9f * weave_sign) * w * w * 4.5f;
            bullet_danger += w;
        }
    }

    /* 2. Kite enemies at optimal range */
    uint32_t e_count = fge_query_circle(pos.x, pos.y, 380.0f, LAYER_ENEMY);
    uint32_t close_enemies = 0u;
    for (uint32_t i = 0; i < e_count; ++i) {
        FgeEntity e = fge_get_query_entity(i);
        if (!fge_entity_is_alive(e)) {
            continue;
        }
        FgeVec2 ep = fge_get_position(e);
        float dx = pos.x - ep.x;
        float dy = pos.y - ep.y;
        float dist = sqrtf(dx * dx + dy * dy + 16.0f);
        float inv = 1.0f / dist;
        if (dist < 185.0f) {
            float w = (185.0f - dist) / 185.0f;
            steer_x += (dx * inv * 1.8f - dy * inv * 0.7f * weave_sign) * w * 3.0f;
            steer_y += (dy * inv * 1.8f + dx * inv * 0.7f * weave_sign) * w * 3.0f;
            close_enemies += 1u;
        } else if (dist > 260.0f && bullet_danger < 1.2f) {
            /* Gentle orbit / approach */
            steer_x += (-dx * inv * 0.35f - dy * inv * 0.45f * weave_sign);
            steer_y += (-dy * inv * 0.35f + dx * inv * 0.45f * weave_sign);
        }
    }

    /* 3. Seek XP gems when safe */
    if (bullet_danger < 1.6f) {
        uint32_t g_count = fge_query_circle(pos.x, pos.y, 440.0f, LAYER_PICKUP);
        float best_g_dist = 1e9f;
        float gx_best = 0.0f;
        float gy_best = 0.0f;
        bool has_gem = false;
        for (uint32_t i = 0; i < g_count; ++i) {
            FgeEntity gem = fge_get_query_entity(i);
            if (!fge_entity_is_alive(gem)) {
                continue;
            }
            FgeVec2 gp = fge_get_position(gem);
            float dx = gp.x - pos.x;
            float dy = gp.y - pos.y;
            float d2 = dx * dx + dy * dy;
            if (d2 < best_g_dist) {
                best_g_dist = d2;
                gx_best = dx;
                gy_best = dy;
                has_gem = true;
            }
        }
        if (has_gem && best_g_dist > 16.0f) {
            float inv = 1.0f / sqrtf(best_g_dist);
            steer_x += gx_best * inv * 1.45f;
            steer_y += gy_best * inv * 1.45f;
        }
    }

    /* 4. Gentle center orbit + strong soft-wall repulsion */
    float cx = pos.x - NEON_ARENA_HALF;
    float cy = pos.y - NEON_ARENA_HALF;
    float cdist = sqrtf(cx * cx + cy * cy + 1.0f);
    if (cdist > 480.0f) {
        float pull = (cdist - 480.0f) / 600.0f;
        steer_x -= (cx / cdist) * pull * 2.2f;
        steer_y -= (cy / cdist) * pull * 2.2f;
    }
    /* Tangential cruise so ship never stalls */
    steer_x += (-cy / cdist) * 0.55f + cosf(g_game.elapsed * 0.9f) * 0.35f;
    steer_y += (cx / cdist) * 0.55f + sinf(g_game.elapsed * 1.1f) * 0.35f;

    float wall_pad = 220.0f;
    if (pos.x < wall_pad) {
        steer_x += (wall_pad - pos.x) / wall_pad * 4.0f;
    } else if (pos.x > NEON_ARENA_SIZE - wall_pad) {
        steer_x -= (pos.x - (NEON_ARENA_SIZE - wall_pad)) / wall_pad * 4.0f;
    }
    if (pos.y < wall_pad) {
        steer_y += (wall_pad - pos.y) / wall_pad * 4.0f;
    } else if (pos.y > NEON_ARENA_SIZE - wall_pad) {
        steer_y -= (pos.y - (NEON_ARENA_SIZE - wall_pad)) / wall_pad * 4.0f;
    }

    *out_mx = steer_x;
    *out_my = steer_y;
    *out_dash = (bullet_danger > 2.2f && g_dash_cooldown <= 0.0f);
    *out_nova = ((b_count >= 14u || close_enemies >= 6u) && g_nova_cooldown <= 0.0f);
}

void player_init(float start_x, float start_y, float start_hp) {
    g_idle_timer = NEON_AUTOPILOT_DELAY;
    g_dash_timer = 0.0f;
    g_dash_cooldown = 0.0f;
    g_nova_cooldown = 2.0f;
    g_thruster_timer = 0.0f;
    g_ring_angle = 0.0f;
    g_heading_angle = -NEON_PI * 0.5f;
    g_mouse_active_timer = 0.0f;
    g_mouse_initialized = false;

    g_game.player_max_hp = 100.0f;
    g_game.player_hp = (start_hp > 0.0f) ? start_hp : 100.0f;
    g_game.player_invuln = 0.8f;
    g_game.player_hit_flash = 0.0f;
    g_game.player_pos = fge_vec2(start_x, start_y);
    g_game.player_aim_dir = fge_vec2(0.0f, -1.0f);
    g_game.autopilot_active = true;
    g_game.dash_ready_frac = 1.0f;
    g_game.nova_ready_frac = 0.75f;

    g_hull = fge_spawn_sprite(start_x, start_y, 24.0f, 24.0f, fge_rgba(65, 235, 255, 255));
    fge_set_collider(g_hull, 10.0f, LAYER_PLAYER);
    fge_set_hp(g_hull, g_game.player_hp);
    fge_set_tag(g_hull, TAG_PLAYER);
    fge_set_z_order(g_hull, 10.0f);
    fge_set_flags(g_hull, fge_get_flags(g_hull) | FGE_FLAG_Y_SORT);

    g_nose = fge_spawn_sprite(start_x, start_y - 12.0f, 18.0f, 9.0f, fge_rgba(230, 252, 255, 255));
    fge_set_tag(g_nose, TAG_PLAYER_PART);
    fge_set_z_order(g_nose, 10.5f);
    fge_set_flags(g_nose, FGE_FLAG_VISIBLE);

    g_wing_l = fge_spawn_sprite(start_x - 14.0f, start_y, 14.0f, 22.0f, fge_rgba(35, 130, 255, 255));
    fge_set_tag(g_wing_l, TAG_PLAYER_PART);
    fge_set_z_order(g_wing_l, 9.5f);
    fge_set_flags(g_wing_l, FGE_FLAG_VISIBLE);

    g_wing_r = fge_spawn_sprite(start_x + 14.0f, start_y, 14.0f, 22.0f, fge_rgba(35, 130, 255, 255));
    fge_set_tag(g_wing_r, TAG_PLAYER_PART);
    fge_set_z_order(g_wing_r, 9.5f);
    fge_set_flags(g_wing_r, FGE_FLAG_VISIBLE);

    for (uint32_t i = 0; i < 4u; ++i) {
        g_core_ring[i] = fge_spawn_sprite(start_x, start_y, 11.0f, 3.0f, fge_rgba(120, 245, 255, 200));
        fge_set_tag(g_core_ring[i], TAG_PLAYER_PART);
        fge_set_z_order(g_core_ring[i], 11.0f);
        fge_set_flags(g_core_ring[i], FGE_FLAG_VISIBLE);
    }

    g_focus_dot = fge_spawn_sprite(start_x, start_y, 6.0f, 6.0f, fge_rgba(255, 110, 215, 230));
    fge_set_tag(g_focus_dot, TAG_PLAYER_PART);
    fge_set_z_order(g_focus_dot, 12.0f);
    fge_set_flags(g_focus_dot, FGE_FLAG_VISIBLE);
}

void player_update(float dt) {
    if (!fge_entity_is_alive(g_hull)) {
        return;
    }

    if (g_game.player_invuln > 0.0f) {
        g_game.player_invuln -= dt;
        if (g_game.player_invuln < 0.0f) {
            g_game.player_invuln = 0.0f;
        }
    }
    if (g_game.player_hit_flash > 0.0f) {
        g_game.player_hit_flash -= dt;
        if (g_game.player_hit_flash < 0.0f) {
            g_game.player_hit_flash = 0.0f;
        }
    }
    if (g_dash_cooldown > 0.0f) {
        g_dash_cooldown -= dt;
        if (g_dash_cooldown < 0.0f) {
            g_dash_cooldown = 0.0f;
        }
    }
    if (g_nova_cooldown > 0.0f) {
        g_nova_cooldown -= dt;
        if (g_nova_cooldown < 0.0f) {
            g_nova_cooldown = 0.0f;
        }
    }
    if (g_dash_timer > 0.0f) {
        g_dash_timer -= dt;
        if (g_dash_timer < 0.0f) {
            g_dash_timer = 0.0f;
        }
    }

    g_game.dash_ready_frac = 1.0f - (g_dash_cooldown / PLAYER_DASH_CD);
    g_game.nova_ready_frac = 1.0f - (g_nova_cooldown / PLAYER_NOVA_CD);
    g_ring_angle += dt * (2.6f + (float)g_game.player_level * 0.25f);

    FgeVec2 pos = fge_get_position(g_hull);

    /* Gather manual movement & action inputs */
    float move_x = 0.0f;
    float move_y = 0.0f;
    bool any_move_key = false;

    if (fge_key_down(FGE_KEY_A) || fge_key_down(FGE_KEY_LEFT)) {
        move_x -= 1.0f;
        any_move_key = true;
    }
    if (fge_key_down(FGE_KEY_D) || fge_key_down(FGE_KEY_RIGHT)) {
        move_x += 1.0f;
        any_move_key = true;
    }
    if (fge_key_down(FGE_KEY_W) || fge_key_down(FGE_KEY_UP)) {
        move_y -= 1.0f;
        any_move_key = true;
    }
    if (fge_key_down(FGE_KEY_S) || fge_key_down(FGE_KEY_DOWN)) {
        move_y += 1.0f;
        any_move_key = true;
    }

    bool focus_mode = fge_key_down(FGE_KEY_LSHIFT) || fge_key_down(FGE_KEY_RSHIFT);
    bool dash_pressed = fge_key_pressed(FGE_KEY_LSHIFT) || fge_key_pressed(FGE_KEY_RSHIFT);
    bool nova_pressed = fge_key_pressed(FGE_KEY_SPACE) || fge_key_pressed(FGE_KEY_Z);

    if (any_move_key) {
        g_idle_timer = 0.0f;
    } else {
        g_idle_timer += dt;
    }
    g_game.autopilot_active = (g_idle_timer >= NEON_AUTOPILOT_DELAY);

    /* Track mouse activity for manual cursor aiming */
    FgeVec2 mpos = fge_mouse_pos();
    if (!g_mouse_initialized) {
        g_last_mouse = mpos;
        g_mouse_initialized = true;
    } else {
        float mdx = mpos.x - g_last_mouse.x;
        float mdy = mpos.y - g_last_mouse.y;
        if ((mdx * mdx + mdy * mdy) > 1.0f || fge_mouse_down(FGE_MOUSE_LEFT)) {
            g_mouse_active_timer = 4.0f;
            g_last_mouse = mpos;
        }
    }
    if (g_mouse_active_timer > 0.0f) {
        g_mouse_active_timer -= dt;
    }

    if (g_game.autopilot_active) {
        bool ai_dash = false;
        bool ai_nova = false;
        compute_autopilot_move(pos, &move_x, &move_y, &ai_dash, &ai_nova);
        if (ai_dash) {
            dash_pressed = true;
        }
        if (ai_nova) {
            nova_pressed = true;
        }
    }

    if (nova_pressed && g_nova_cooldown <= 0.0f) {
        weapons_trigger_nova();
        g_nova_cooldown = PLAYER_NOVA_CD;
    }

    if (dash_pressed && g_dash_cooldown <= 0.0f) {
        float len_sq = move_x * move_x + move_y * move_y;
        if (len_sq > 0.001f) {
            float inv = 1.0f / sqrtf(len_sq);
            g_dash_dir = fge_vec2(move_x * inv, move_y * inv);
        } else {
            g_dash_dir = g_game.player_aim_dir;
        }
        g_dash_timer = PLAYER_DASH_DUR;
        g_dash_cooldown = PLAYER_DASH_CD;
        if (g_game.player_invuln < 0.25f) {
            g_game.player_invuln = 0.25f;
        }
        fx_spawn_ring(pos.x, pos.y, 12u, 220.0f, 7.0f, fge_rgba(120, 245, 255, 230));
        game_add_camera_shake(0.18f);
    }

    float vx = 0.0f;
    float vy = 0.0f;
    if (g_dash_timer > 0.0f) {
        vx = g_dash_dir.x * PLAYER_DASH_SPEED;
        vy = g_dash_dir.y * PLAYER_DASH_SPEED;
    } else {
        float len_sq = move_x * move_x + move_y * move_y;
        if (len_sq > 0.0001f) {
            float inv = 1.0f / sqrtf(len_sq);
            float spd = focus_mode ? PLAYER_FOCUS_SPEED : PLAYER_SPEED;
            if (!g_game.autopilot_active) {
                spd *= 1.0f;
            }
            vx = move_x * inv * spd;
            vy = move_y * inv * spd;
        }
    }

    /* Clamp inside arena boundaries */
    float clamped_x = pos.x;
    float clamped_y = pos.y;
    if (clamped_x < NEON_ARENA_MARGIN) {
        clamped_x = NEON_ARENA_MARGIN;
        if (vx < 0.0f) {
            vx = 0.0f;
        }
    } else if (clamped_x > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) {
        clamped_x = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        if (vx > 0.0f) {
            vx = 0.0f;
        }
    }
    if (clamped_y < NEON_ARENA_MARGIN) {
        clamped_y = NEON_ARENA_MARGIN;
        if (vy < 0.0f) {
            vy = 0.0f;
        }
    } else if (clamped_y > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) {
        clamped_y = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        if (vy > 0.0f) {
            vy = 0.0f;
        }
    }
    if (clamped_x != pos.x || clamped_y != pos.y) {
        fge_set_position(g_hull, clamped_x, clamped_y);
        pos.x = clamped_x;
        pos.y = clamped_y;
    }

    fge_set_velocity(g_hull, vx, vy);
    g_game.player_pos = pos;

    /* Aim direction: mouse world pos when active, else auto-aim nearest enemy */
    if (!g_game.autopilot_active && g_mouse_active_timer > 0.0f) {
        FgeCamera cam = fge_camera_get();
        float z = (cam.zoom > 0.01f) ? cam.zoom : 1.0f;
        float wx = cam.center_x + (mpos.x - cam.viewport_w * 0.5f) / z;
        float wy = cam.center_y + (mpos.y - cam.viewport_h * 0.5f) / z;
        float dx = wx - pos.x;
        float dy = wy - pos.y;
        float d2 = dx * dx + dy * dy;
        if (d2 > 4.0f) {
            float inv = 1.0f / sqrtf(d2);
            g_game.player_aim_dir = fge_vec2(dx * inv, dy * inv);
        }
    } else {
        g_game.player_aim_dir = compute_auto_aim(pos);
    }

    /* Engine thruster particles */
    g_thruster_timer -= dt;
    float spd_sq = vx * vx + vy * vy;
    if (g_thruster_timer <= 0.0f && spd_sq > 900.0f) {
        float inv_s = 1.0f / sqrtf(spd_sq);
        float back_x = -vx * inv_s;
        float back_y = -vy * inv_s;
        FgeColor tcol = (g_dash_timer > 0.0f)
                            ? fge_rgba(220, 255, 255, 240)
                            : fge_rgba(45, 195, 255, 210);
        fx_spawn_thruster(pos.x + back_x * 14.0f, pos.y + back_y * 14.0f, back_x, back_y, tcol);
        g_thruster_timer = (g_dash_timer > 0.0f) ? 0.014f : 0.032f;
    }

    update_ship_visuals(pos, fge_vec2(vx, vy), focus_mode);
}

void player_apply_damage(float dmg, FgeVec2 knock_from) {
    if (g_game.player_invuln > 0.0f || !fge_entity_is_alive(g_hull)) {
        return;
    }

    g_game.player_hp -= dmg;
    g_game.player_invuln = PLAYER_HIT_INVULN;
    g_game.player_hit_flash = 0.16f;
    game_add_camera_shake(0.45f);

    FgeVec2 pos = fge_get_position(g_hull);
    fx_spawn_burst(pos.x, pos.y, 14u, 90.0f, 280.0f, 7.0f, fge_rgba(255, 80, 120, 255));

    /* Subtle knockback away from hit source */
    float dx = pos.x - knock_from.x;
    float dy = pos.y - knock_from.y;
    float d2 = dx * dx + dy * dy;
    if (d2 > 1.0f) {
        float inv = 1.0f / sqrtf(d2);
        float nx = pos.x + dx * inv * 18.0f;
        float ny = pos.y + dy * inv * 18.0f;
        if (nx < NEON_ARENA_MARGIN) nx = NEON_ARENA_MARGIN;
        if (nx > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) nx = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        if (ny < NEON_ARENA_MARGIN) ny = NEON_ARENA_MARGIN;
        if (ny > NEON_ARENA_SIZE - NEON_ARENA_MARGIN) ny = NEON_ARENA_SIZE - NEON_ARENA_MARGIN;
        fge_set_position(g_hull, nx, ny);
    }

    if (g_game.player_hp <= 0.0f) {
        /* Emergency Overload Shield: clears nearby bullets & restores hull so showcase continues */
        g_game.player_hp = g_game.player_max_hp;
        g_game.player_invuln = 1.8f;
        weapons_trigger_nova();
    }
    fge_set_hp(g_hull, g_game.player_hp);
}

void player_heal_full(void) {
    g_game.player_hp = g_game.player_max_hp;
    if (fge_entity_is_alive(g_hull)) {
        fge_set_hp(g_hull, g_game.player_hp);
    }
    FgeVec2 pos = player_get_position();
    fx_spawn_ring(pos.x, pos.y, 20u, 220.0f, 8.0f, fge_rgba(80, 255, 160, 255));
}

void player_shutdown(void) {
    FgeEntity parts[6] = { g_hull, g_nose, g_wing_l, g_wing_r, g_focus_dot, fge_entity_null() };
    for (uint32_t i = 0; i < 5u; ++i) {
        if (fge_entity_is_alive(parts[i])) {
            fge_entity_despawn(parts[i]);
        }
    }
    for (uint32_t i = 0; i < 4u; ++i) {
        if (fge_entity_is_alive(g_core_ring[i])) {
            fge_entity_despawn(g_core_ring[i]);
        }
        g_core_ring[i] = fge_entity_null();
    }
    g_hull = fge_entity_null();
    g_nose = fge_entity_null();
    g_wing_l = fge_entity_null();
    g_wing_r = fge_entity_null();
    g_focus_dot = fge_entity_null();
}

FgeEntity player_get_entity(void) {
    return g_hull;
}

FgeVec2 player_get_position(void) {
    if (!fge_entity_is_alive(g_hull)) {
        return g_game.player_pos;
    }
    return fge_get_position(g_hull);
}
