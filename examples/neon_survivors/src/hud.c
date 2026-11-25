#include "neon_shared.h"

#include <math.h>

#define SCORE_DIGITS 7u
#define LEVEL_DIGITS 2u

static const uint8_t SEVEN_SEG_LUT[10] = {
    0x3Fu, /* 0 */
    0x06u, /* 1 */
    0x5Bu, /* 2 */
    0x4Fu, /* 3 */
    0x66u, /* 4 */
    0x6Du, /* 5 */
    0x7Du, /* 6 */
    0x07u, /* 7 */
    0x7Fu, /* 8 */
    0x6Fu  /* 9 */
};

/* HUD bars */
static FgeEntity g_hp_frame;
static FgeEntity g_hp_bg;
static FgeEntity g_hp_fill;

static FgeEntity g_xp_frame;
static FgeEntity g_xp_bg;
static FgeEntity g_xp_fill;

static FgeEntity g_dash_bg;
static FgeEntity g_dash_fill;
static FgeEntity g_nova_bg;
static FgeEntity g_nova_fill;

static FgeEntity g_boss_frame;
static FgeEntity g_boss_bg;
static FgeEntity g_boss_fill;

static FgeEntity g_auto_badge;

/* 7-segment digital counters */
static FgeEntity g_score_segs[SCORE_DIGITS][7];
static FgeEntity g_level_segs[LEVEL_DIGITS][7];

static FgeEntity spawn_hud_rect(float z, FgeColor col) {
    FgeEntity e = fge_spawn_sprite(0.0f, 0.0f, 10.0f, 10.0f, col);
    fge_set_tag(e, TAG_HUD);
    fge_set_z_order(e, z);
    /* Clear FGE_FLAG_KINEMATIC so HUD never drifts */
    fge_set_flags(e, FGE_FLAG_VISIBLE);
    return e;
}

static void place_hud_rect(FgeCamera cam, float sx, float sy, float sw, float sh, FgeEntity e, FgeColor col, bool visible) {
    if (!fge_entity_is_alive(e)) {
        return;
    }
    if (!visible) {
        fge_set_flags(e, 0u);
        return;
    }
    float inv_z = (cam.zoom > 0.01f) ? (1.0f / cam.zoom) : 1.0f;
    float wx = cam.center_x + (sx - cam.viewport_w * 0.5f) * inv_z;
    float wy = cam.center_y + (sy - cam.viewport_h * 0.5f) * inv_z;
    fge_set_flags(e, FGE_FLAG_VISIBLE);
    fge_set_position(e, wx, wy);
    fge_set_size(e, sw * inv_z, sh * inv_z);
    fge_set_color(e, col);
}

static void update_seven_seg_digit(
    FgeCamera cam,
    FgeEntity segs[7],
    uint32_t digit,
    float cx,
    float cy,
    float w,
    float h,
    float t,
    FgeColor on_col,
    FgeColor off_col
) {
    uint8_t mask = SEVEN_SEG_LUT[digit % 10u];
    float hw = w * 0.5f;
    float hh = h * 0.5f;
    float qh = h * 0.25f;

    float sx[7] = { cx, cx + hw, cx + hw, cx, cx - hw, cx - hw, cx };
    float sy[7] = { cy - hh, cy - qh, cy + qh, cy + hh, cy + qh, cy - qh, cy };
    float sw[7] = { w - t * 0.6f, t, t, w - t * 0.6f, t, t, w - t * 0.6f };
    float sh[7] = { t, hh - t * 0.6f, hh - t * 0.6f, t, hh - t * 0.6f, hh - t * 0.6f, t };

    for (uint32_t s = 0; s < 7u; ++s) {
        bool lit = ((mask >> s) & 1u) != 0u;
        place_hud_rect(cam, sx[s], sy[s], sw[s], sh[s], segs[s], lit ? on_col : off_col, true);
    }
}

void hud_init(void) {
    g_hp_frame = spawn_hud_rect(99.0f, fge_rgba(35, 55, 85, 225));
    g_hp_bg = spawn_hud_rect(99.5f, fge_rgba(12, 16, 28, 235));
    g_hp_fill = spawn_hud_rect(100.0f, fge_rgba(60, 240, 140, 255));

    g_xp_frame = spawn_hud_rect(99.0f, fge_rgba(35, 65, 95, 225));
    g_xp_bg = spawn_hud_rect(99.5f, fge_rgba(12, 16, 28, 235));
    g_xp_fill = spawn_hud_rect(100.0f, fge_rgba(55, 230, 255, 255));

    g_dash_bg = spawn_hud_rect(99.5f, fge_rgba(14, 20, 34, 220));
    g_dash_fill = spawn_hud_rect(100.0f, fge_rgba(90, 210, 255, 245));
    g_nova_bg = spawn_hud_rect(99.5f, fge_rgba(14, 20, 34, 220));
    g_nova_fill = spawn_hud_rect(100.0f, fge_rgba(255, 110, 245, 245));

    g_boss_frame = spawn_hud_rect(99.0f, fge_rgba(90, 25, 65, 235));
    g_boss_bg = spawn_hud_rect(99.5f, fge_rgba(18, 10, 24, 240));
    g_boss_fill = spawn_hud_rect(100.0f, fge_rgba(255, 60, 175, 255));

    g_auto_badge = spawn_hud_rect(100.0f, fge_rgba(90, 255, 185, 230));
    fge_set_rotation(g_auto_badge, NEON_PI * 0.25f);

    for (uint32_t d = 0; d < SCORE_DIGITS; ++d) {
        for (uint32_t s = 0; s < 7u; ++s) {
            g_score_segs[d][s] = spawn_hud_rect(100.0f, fge_rgba(80, 245, 255, 255));
        }
    }
    for (uint32_t d = 0; d < LEVEL_DIGITS; ++d) {
        for (uint32_t s = 0; s < 7u; ++s) {
            g_level_segs[d][s] = spawn_hud_rect(100.0f, fge_rgba(255, 220, 80, 255));
        }
    }
}

void hud_update(float dt) {
    (void)dt;
    FgeCamera cam = fge_camera_get();
    float vw = (cam.viewport_w > 1.0f) ? cam.viewport_w : NEON_VIEW_W;

    /* 1. Player HP Bar (top-left) */
    float hp_frac = (g_game.player_max_hp > 0.0f) ? (g_game.player_hp / g_game.player_max_hp) : 0.0f;
    if (hp_frac < 0.0f) hp_frac = 0.0f;
    if (hp_frac > 1.0f) hp_frac = 1.0f;

    FgeColor hp_col = fge_rgba(65, 245, 135, 255);
    if (hp_frac < 0.28f) {
        hp_col = fge_rgba(255, 65, 85, 255);
    } else if (hp_frac < 0.58f) {
        hp_col = fge_rgba(255, 210, 65, 255);
    }

    float bar_x = 26.0f;
    float bar_w = 236.0f;
    place_hud_rect(cam, bar_x + bar_w * 0.5f, 30.0f, bar_w + 6.0f, 22.0f, g_hp_frame, fge_rgba(40, 70, 110, 225), true);
    place_hud_rect(cam, bar_x + bar_w * 0.5f, 30.0f, bar_w, 16.0f, g_hp_bg, fge_rgba(12, 16, 28, 240), true);
    float hp_fill_w = bar_w * hp_frac;
    place_hud_rect(cam, bar_x + hp_fill_w * 0.5f, 30.0f, hp_fill_w > 0.5f ? hp_fill_w : 0.5f, 16.0f, g_hp_fill, hp_col, hp_frac > 0.0f);

    /* 2. XP Progress Bar */
    float xp_frac = (g_game.xp_next > 0.0f) ? (g_game.xp / g_game.xp_next) : 1.0f;
    if (g_game.player_level >= NEON_MAX_LEVEL) xp_frac = 1.0f;
    if (xp_frac < 0.0f) xp_frac = 0.0f;
    if (xp_frac > 1.0f) xp_frac = 1.0f;

    place_hud_rect(cam, bar_x + bar_w * 0.5f, 54.0f, bar_w + 4.0f, 12.0f, g_xp_frame, fge_rgba(35, 80, 120, 220), true);
    place_hud_rect(cam, bar_x + bar_w * 0.5f, 54.0f, bar_w, 8.0f, g_xp_bg, fge_rgba(10, 14, 24, 240), true);
    float xp_fill_w = bar_w * xp_frac;
    place_hud_rect(
        cam,
        bar_x + xp_fill_w * 0.5f,
        54.0f,
        xp_fill_w > 0.5f ? xp_fill_w : 0.5f,
        8.0f,
        g_xp_fill,
        fge_rgba(55, 235, 255, 255),
        xp_frac > 0.0f
    );

    /* 3. Dash & Nova Mini Readiness Bars */
    float mini_w = 114.0f;
    float df = g_game.dash_ready_frac;
    if (df < 0.0f) df = 0.0f;
    if (df > 1.0f) df = 1.0f;
    place_hud_rect(cam, bar_x + mini_w * 0.5f, 70.0f, mini_w, 5.0f, g_dash_bg, fge_rgba(14, 20, 34, 220), true);
    place_hud_rect(
        cam,
        bar_x + (mini_w * df) * 0.5f,
        70.0f,
        (mini_w * df) > 0.5f ? (mini_w * df) : 0.5f,
        5.0f,
        g_dash_fill,
        (df >= 0.99f) ? fge_rgba(120, 245, 255, 255) : fge_rgba(60, 140, 210, 210),
        df > 0.01f
    );

    float nf = g_game.nova_ready_frac;
    if (nf < 0.0f) nf = 0.0f;
    if (nf > 1.0f) nf = 1.0f;
    float nova_x = bar_x + mini_w + 8.0f;
    place_hud_rect(cam, nova_x + mini_w * 0.5f, 70.0f, mini_w, 5.0f, g_nova_bg, fge_rgba(14, 20, 34, 220), true);
    place_hud_rect(
        cam,
        nova_x + (mini_w * nf) * 0.5f,
        70.0f,
        (mini_w * nf) > 0.5f ? (mini_w * nf) : 0.5f,
        5.0f,
        g_nova_fill,
        (nf >= 0.99f) ? fge_rgba(255, 130, 255, 255) : fge_rgba(165, 70, 180, 210),
        nf > 0.01f
    );

    /* 4. Boss HP Bar (top-center, only visible when boss active) */
    if (g_game.boss_active && g_game.boss_max_hp > 0.0f) {
        float bfrac = g_game.boss_hp / g_game.boss_max_hp;
        if (bfrac < 0.0f) bfrac = 0.0f;
        if (bfrac > 1.0f) bfrac = 1.0f;
        float b_w = 420.0f;
        float b_left = vw * 0.5f - b_w * 0.5f;
        float fill_w = b_w * bfrac;
        uint8_t pulse_g = (uint8_t)(55.0f + 40.0f * sinf(g_game.elapsed * 9.0f));
        place_hud_rect(cam, vw * 0.5f, 32.0f, b_w + 8.0f, 22.0f, g_boss_frame, fge_rgba(130, 35, 95, 240), true);
        place_hud_rect(cam, vw * 0.5f, 32.0f, b_w, 14.0f, g_boss_bg, fge_rgba(20, 10, 28, 245), true);
        place_hud_rect(
            cam,
            b_left + fill_w * 0.5f,
            32.0f,
            fill_w > 0.5f ? fill_w : 0.5f,
            14.0f,
            g_boss_fill,
            fge_rgba(255, pulse_g, 185, 255),
            bfrac > 0.0f
        );
    } else {
        place_hud_rect(cam, 0.0f, 0.0f, 1.0f, 1.0f, g_boss_frame, fge_rgba(0, 0, 0, 0), false);
        place_hud_rect(cam, 0.0f, 0.0f, 1.0f, 1.0f, g_boss_bg, fge_rgba(0, 0, 0, 0), false);
        place_hud_rect(cam, 0.0f, 0.0f, 1.0f, 1.0f, g_boss_fill, fge_rgba(0, 0, 0, 0), false);
    }

    /* 5. 7-Segment Level Counter (next to HP/XP bar) */
    uint32_t lvl_val = g_game.player_level;
    FgeColor lvl_on = fge_rgba(255, 225, 85, 255);
    FgeColor lvl_off = fge_rgba(45, 42, 22, 55);
    for (uint32_t d = 0; d < LEVEL_DIGITS; ++d) {
        uint32_t div = (d == 0u) ? 10u : 1u;
        uint32_t digit = (lvl_val / div) % 10u;
        update_seven_seg_digit(
            cam,
            g_level_segs[d],
            digit,
            288.0f + (float)d * 24.0f,
            42.0f,
            14.0f,
            28.0f,
            3.2f,
            lvl_on,
            lvl_off
        );
    }

    /* 6. 7-Segment Score Counter (top-right) */
    uint32_t score_val = g_game.score;
    FgeColor sc_on = fge_rgba(75, 245, 255, 255);
    FgeColor sc_off = fge_rgba(22, 48, 62, 50);
    float score_start_x = vw - 28.0f - (float)(SCORE_DIGITS - 1u) * 24.0f;
    uint32_t pow10 = 1000000u;
    for (uint32_t d = 0; d < SCORE_DIGITS; ++d) {
        uint32_t digit = (score_val / pow10) % 10u;
        update_seven_seg_digit(
            cam,
            g_score_segs[d],
            digit,
            score_start_x + (float)d * 24.0f,
            38.0f,
            14.0f,
            28.0f,
            3.2f,
            sc_on,
            sc_off
        );
        pow10 /= 10u;
    }

    /* 7. Autopilot pulsing indicator diamond under score */
    if (g_game.autopilot_active) {
        uint8_t pulse_a = (uint8_t)(160.0f + 90.0f * sinf(g_game.elapsed * 5.0f));
        place_hud_rect(
            cam,
            vw - 32.0f,
            72.0f,
            10.0f,
            10.0f,
            g_auto_badge,
            fge_rgba(90, 255, 185, pulse_a),
            true
        );
        if (fge_entity_is_alive(g_auto_badge)) {
            fge_set_rotation(g_auto_badge, g_game.elapsed * 2.2f);
        }
    } else {
        place_hud_rect(cam, 0.0f, 0.0f, 1.0f, 1.0f, g_auto_badge, fge_rgba(0, 0, 0, 0), false);
    }
}

void hud_shutdown(void) {
    FgeEntity single_hud[14] = {
        g_hp_frame, g_hp_bg, g_hp_fill,
        g_xp_frame, g_xp_bg, g_xp_fill,
        g_dash_bg, g_dash_fill, g_nova_bg, g_nova_fill,
        g_boss_frame, g_boss_bg, g_boss_fill,
        g_auto_badge
    };
    for (uint32_t i = 0; i < 14u; ++i) {
        if (fge_entity_is_alive(single_hud[i])) {
            fge_entity_despawn(single_hud[i]);
        }
    }
    for (uint32_t d = 0; d < SCORE_DIGITS; ++d) {
        for (uint32_t s = 0; s < 7u; ++s) {
            if (fge_entity_is_alive(g_score_segs[d][s])) {
                fge_entity_despawn(g_score_segs[d][s]);
            }
            g_score_segs[d][s] = fge_entity_null();
        }
    }
    for (uint32_t d = 0; d < LEVEL_DIGITS; ++d) {
        for (uint32_t s = 0; s < 7u; ++s) {
            if (fge_entity_is_alive(g_level_segs[d][s])) {
                fge_entity_despawn(g_level_segs[d][s]);
            }
            g_level_segs[d][s] = fge_entity_null();
        }
    }
}
