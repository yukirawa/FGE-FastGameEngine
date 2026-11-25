#include "rpg_shared.h"

#include <math.h>
#include <string.h>

RpgGameState g_rpg;

float rpg_clamp_f32(float v, float lo, float hi) {
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

void rpg_add_camera_shake(float amount) {
    g_rpg.shake_trauma = rpg_clamp_f32(g_rpg.shake_trauma + amount, 0.0f, 1.0f);
}

void rpg_add_popup(float wx, float wy, float font_size, FgeColor color, const char *text) {
    if (!text || text[0] == '\0') {
        return;
    }
    uint32_t slot = 0u;
    float oldest = -1.0f;
    for (uint32_t i = 0; i < MAX_WORLD_POPUPS; ++i) {
        if (!g_rpg.popups[i].active) {
            slot = i;
            oldest = 9999.0f;
            break;
        }
        if (g_rpg.popups[i].timer > oldest) {
            oldest = g_rpg.popups[i].timer;
            slot = i;
        }
    }
    WorldPopup *p = &g_rpg.popups[slot];
    p->active = true;
    p->x = wx;
    p->y = wy;
    p->vy = -34.0f;
    p->timer = 0.0f;
    p->duration = 1.05f;
    p->font_size = font_size;
    p->color = color;

    size_t i = 0u;
    while (i + 1u < sizeof(p->text) && text[i] != '\0') {
        p->text[i] = text[i];
        i++;
    }
    p->text[i] = '\0';
}

void rpg_spawn_burst(float x, float y, uint32_t count, float speed, float ttl, float size, FgeColor col) {
    for (uint32_t i = 0; i < count; ++i) {
        float ang = ((float)i / (float)count) * RPG_TAU + fge_rand_f32() * 0.35f;
        float spd = speed * (0.55f + fge_rand_f32() * 0.65f);
        FgeEntity e = fge_spawn_sprite(x, y, size, size, col);
        fge_set_velocity(e, cosf(ang) * spd, sinf(ang) * spd);
        fge_set_rotation(e, ang);
        fge_set_angular_velocity(e, 4.0f);
        fge_set_z_order(e, 1.9f);
        fge_set_tag(e, TAG_FX);
        fge_set_flags(e, FGE_FLAG_VISIBLE | FGE_FLAG_KINEMATIC);
        fge_set_ttl(e, ttl);
    }
}

void rpg_sync_persisted_params(void) {
    fge_param_set(PARAM_PERSIST_HERO_X, g_rpg.hero_pos.x);
    fge_param_set(PARAM_PERSIST_HERO_Y, g_rpg.hero_pos.y);
    fge_param_set(PARAM_PERSIST_HERO_HP, g_rpg.hero_hp);
    fge_param_set(PARAM_PERSIST_LEVEL, (float)g_rpg.hero_level);
    fge_param_set(PARAM_PERSIST_EXP, (float)g_rpg.hero_exp);
    fge_param_set(PARAM_PERSIST_GOLD, (float)g_rpg.gold);
    fge_param_set(PARAM_PERSIST_SLAIN, (float)g_rpg.monsters_slain);
    fge_param_set(PARAM_PERSIST_QUEST, (float)g_rpg.quest_stage);
}

static bool any_manual_input(void) {
    static const uint32_t keys[] = {
        FGE_KEY_W, FGE_KEY_A, FGE_KEY_S, FGE_KEY_D,
        FGE_KEY_UP, FGE_KEY_DOWN, FGE_KEY_LEFT, FGE_KEY_RIGHT,
        FGE_KEY_SPACE, FGE_KEY_Z, FGE_KEY_X, FGE_KEY_E,
        FGE_KEY_ENTER, FGE_KEY_LSHIFT
    };
    uint32_t n = (uint32_t)(sizeof(keys) / sizeof(keys[0]));
    for (uint32_t i = 0; i < n; ++i) {
        if (fge_key_down(keys[i]) || fge_key_pressed(keys[i])) {
            return true;
        }
    }
    if (fge_mouse_down(FGE_MOUSE_LEFT)) {
        return true;
    }
    return false;
}

static FgeVec2 compute_bridge_aware_waypoint(float tx, float ty) {
    float hx = g_rpg.hero_pos.x;
    float hy = g_rpg.hero_pos.y;
    bool hero_south = (hy > RPG_RIVER_MAX_Y + 6.0f);
    bool hero_north = (hy < RPG_RIVER_MIN_Y - 6.0f);
    bool target_north = (ty < RPG_RIVER_MIN_Y - 6.0f);
    bool target_south = (ty > RPG_RIVER_MAX_Y + 6.0f);

    if (hero_south && target_north) {
        if (fabsf(hx - 1100.0f) > 36.0f) {
            return fge_vec2(1100.0f, 1125.0f);
        }
        return fge_vec2(1100.0f, 975.0f);
    }
    if (hero_north && target_south) {
        if (fabsf(hx - 1100.0f) > 36.0f) {
            return fge_vec2(1100.0f, 965.0f);
        }
        return fge_vec2(1100.0f, 1125.0f);
    }
    /* Guide cleanly into the Ruins southern gate if approaching from outside */
    if (hy > 530.0f && ty < 480.0f && fabsf(hx - 1100.0f) > 60.0f) {
        return fge_vec2(1100.0f, 545.0f);
    }
    return fge_vec2(tx, ty);
}

static void update_autopilot_ai(float dt) {
    (void)dt;
    if (!g_rpg.autopilot_active || g_rpg.dialogue.active) {
        hero_set_move_intent(0.0f, 0.0f);
        return;
    }

    float hx = g_rpg.hero_pos.x;
    float hy = g_rpg.hero_pos.y;

    /* 1. Visit Village Elder to accept quest */
    if (g_rpg.quest_stage == QUEST_NOT_STARTED) {
        float dx = RPG_ELDER_X - hx;
        float dy = RPG_ELDER_Y - hy;
        if (dx * dx + dy * dy <= 76.0f * 76.0f) {
            hero_set_move_intent(0.0f, 0.0f);
            npc_quest_try_interact();
            return;
        }
        FgeVec2 wp = compute_bridge_aware_waypoint(RPG_ELDER_X, RPG_ELDER_Y + 34.0f);
        hero_set_move_intent(wp.x - hx, wp.y - hy);
        return;
    }

    /* 2. Visit Forest Fairy near bridge if HP < 45% */
    if (g_rpg.hero_hp < g_rpg.hero_max_hp * 0.45f) {
        float fdx = RPG_FAIRY_X - hx;
        float fdy = RPG_FAIRY_Y - hy;
        if (fdx * fdx + fdy * fdy <= 78.0f * 78.0f) {
            hero_set_move_intent(0.0f, 0.0f);
            npc_quest_try_interact();
            return;
        }
        FgeVec2 wp = compute_bridge_aware_waypoint(RPG_FAIRY_X, RPG_FAIRY_Y + 32.0f);
        hero_set_move_intent(wp.x - hx, wp.y - hy);
        return;
    }

    /* 3. Report victory back to Village Elder once Obsidian Golem is defeated */
    if (g_rpg.quest_stage == QUEST_BOSS_DEFEATED) {
        float edx = RPG_ELDER_X - hx;
        float edy = RPG_ELDER_Y - hy;
        if (edx * edx + edy * edy <= 78.0f * 78.0f) {
            hero_set_move_intent(0.0f, 0.0f);
            npc_quest_try_interact();
            return;
        }
        FgeVec2 wp = compute_bridge_aware_waypoint(RPG_ELDER_X, RPG_ELDER_Y + 34.0f);
        hero_set_move_intent(wp.x - hx, wp.y - hy);
        return;
    }

    /* 4. Combat & Exploration in Whispering Woods and Northern Ruins */
    FgeVec2 target_pos = fge_vec2(1100.0f, 1200.0f);
    bool is_boss = false;
    bool ready_for_boss = (g_rpg.hero_level >= 2u || g_rpg.monsters_slain >= 3u || g_rpg.elapsed >= 8.0f);
    float search_range = ready_for_boss ? 280.0f : 560.0f;

    bool has_target = monsters_find_nearest_target(hx, hy, search_range, &target_pos, &is_boss);
    if (!has_target) {
        if (ready_for_boss && !g_rpg.boss_defeated) {
            target_pos = monsters_get_golem_pos();
            has_target = true;
            is_boss = true;
        } else if (hy > 900.0f) {
            target_pos = fge_vec2(1100.0f, 820.0f);
        } else {
            target_pos = fge_vec2(RPG_CRYSTAL_X, RPG_CRYSTAL_Y + 48.0f);
        }
    }

    if (has_target) {
        float dx = target_pos.x - hx;
        float dy = target_pos.y - hy;
        float dist = sqrtf(dx * dx + dy * dy);
        float strike_range = is_boss ? 76.0f : 58.0f;

        /* Dash out of danger if enemy rune bullets are very close */
        if (fge_query_circle(hx, hy, 44.0f, LAYER_ENEMY_BULLET) > 0u) {
            hero_trigger_dash();
        }

        if (dist <= strike_range) {
            if (dist > 0.01f) {
                g_rpg.hero_dir = fge_vec2(dx / dist, dy / dist);
            }
            hero_trigger_slash();
            /* Strafe tangentially around boss while fighting */
            if (is_boss && dist > 0.01f) {
                hero_set_move_intent(-dy / dist, dx / dist);
            } else {
                hero_set_move_intent(0.0f, 0.0f);
            }
            return;
        }
    }

    FgeVec2 wp = compute_bridge_aware_waypoint(target_pos.x, target_pos.y);
    hero_set_move_intent(wp.x - hx, wp.y - hy);
}

static void update_dynamic_bgm(void) {
    uint32_t desired = FGE_BGM_FIELD;
    if (g_rpg.boss_active && !g_rpg.boss_defeated) {
        desired = FGE_BGM_BOSS;
    } else if (g_rpg.hero_pos.y >= RPG_VILLAGE_MIN_Y) {
        desired = FGE_BGM_VILLAGE;
    }
    if (desired != g_rpg.current_bgm) {
        g_rpg.current_bgm = desired;
        fge_bgm_play_preset(desired, 0.42f);
    }
}

static void update_camera(float dt) {
    float follow = 1.0f - expf(-8.5f * dt);
    g_rpg.cam_x += (g_rpg.hero_pos.x - g_rpg.cam_x) * follow;
    g_rpg.cam_y += (g_rpg.hero_pos.y - g_rpg.cam_y) * follow;

    float half_w = (RPG_VIEW_W * 0.5f) / g_rpg.cam_zoom;
    float half_h = (RPG_VIEW_H * 0.5f) / g_rpg.cam_zoom;
    g_rpg.cam_x = rpg_clamp_f32(g_rpg.cam_x, half_w, RPG_WORLD_W - half_w);
    g_rpg.cam_y = rpg_clamp_f32(g_rpg.cam_y, half_h, RPG_WORLD_H - half_h);

    FgeVec2 wheel = fge_mouse_wheel();
    if (wheel.y != 0.0f) {
        g_rpg.cam_zoom = rpg_clamp_f32(g_rpg.cam_zoom + wheel.y * 0.08f, 0.80f, 1.45f);
    }

    float sx = 0.0f;
    float sy = 0.0f;
    if (g_rpg.shake_trauma > 0.0f) {
        g_rpg.shake_time += dt * 40.0f;
        float mag = g_rpg.shake_trauma * g_rpg.shake_trauma * 16.0f;
        sx = sinf(g_rpg.shake_time * 1.4f) * mag;
        sy = cosf(g_rpg.shake_time * 1.8f) * mag;
        g_rpg.shake_trauma -= dt * 2.2f;
        if (g_rpg.shake_trauma < 0.0f) {
            g_rpg.shake_trauma = 0.0f;
        }
    }

    fge_camera_set(g_rpg.cam_x + sx, g_rpg.cam_y + sy, g_rpg.cam_zoom);
}

static void handle_ai_events(void) {
    uint32_t ev_n = fge_poll_ai_events();
    for (uint32_t i = 0; i < ev_n; ++i) {
        FgeAiEvent ev = fge_get_ai_event(i);
        if (ev.kind == AI_EVENT_HEAL_HERO) {
            hero_heal_full();
            fge_audio_play_sfx(FGE_SFX_HEAL, 0.8f, 1.0f);
        } else if (ev.kind == AI_EVENT_SPAWN_GOLD) {
            uint32_t amt = (ev.arg0 > 1.0f) ? (uint32_t)ev.arg0 : 50u;
            g_rpg.gold += amt;
            fge_audio_play_sfx(FGE_SFX_PICKUP, 0.7f, 1.0f);
        }
    }
}

static void reset_rpg_state(float start_x, float start_y, float start_hp) {
    memset(&g_rpg, 0, sizeof(g_rpg));
    g_rpg.initialized = true;
    g_rpg.idle_timer = RPG_AUTOPILOT_DELAY;
    g_rpg.autopilot_active = true;
    g_rpg.hero_level = 1u;
    g_rpg.gold = 0u;
    g_rpg.monsters_slain = 0u;
    g_rpg.hero_hp = start_hp;
    g_rpg.hero_max_hp = 100.0f;
    g_rpg.hero_pos = fge_vec2(start_x, start_y);
    g_rpg.hero_dir = fge_vec2(0.0f, -1.0f);
    g_rpg.quest_stage = QUEST_NOT_STARTED;
    g_rpg.current_bgm = FGE_BGM_NONE;
    g_rpg.cam_x = start_x;
    g_rpg.cam_y = start_y;
    g_rpg.cam_zoom = 1.0f;
}

void fge_init(void) {
    reset_rpg_state(RPG_HERO_SPAWN_X, RPG_HERO_SPAWN_Y, 100.0f);
    fge_audio_set_master_volume(0.8f);
    fge_camera_set(g_rpg.cam_x, g_rpg.cam_y, g_rpg.cam_zoom);

    world_map_init();
    hero_init(RPG_HERO_SPAWN_X, RPG_HERO_SPAWN_Y, 100.0f);
    npc_quest_init();
    monsters_init();

    update_dynamic_bgm();
    rpg_sync_persisted_params();
}

void fge_on_reload(void) {
    if (!g_rpg.initialized) {
        float sx = fge_param_get(PARAM_PERSIST_HERO_X, RPG_HERO_SPAWN_X);
        float sy = fge_param_get(PARAM_PERSIST_HERO_Y, RPG_HERO_SPAWN_Y);
        float shp = fge_param_get(PARAM_PERSIST_HERO_HP, 100.0f);
        uint32_t lv = (uint32_t)fge_param_get(PARAM_PERSIST_LEVEL, 1.0f);
        if (lv < 1u) lv = 1u;
        if (lv > RPG_MAX_LEVEL) lv = RPG_MAX_LEVEL;

        uint32_t all_masks = LAYER_HERO | LAYER_MONSTER | LAYER_SWORD_SLASH | LAYER_ENEMY_BULLET;
        uint32_t cn = fge_query_circle(RPG_WORLD_W * 0.5f, RPG_WORLD_H * 0.5f, 3200.0f, all_masks);
        for (uint32_t i = 0; i < cn; ++i) {
            FgeEntity ce = fge_get_query_entity(i);
            if (fge_entity_is_alive(ce)) {
                fge_entity_despawn(ce);
            }
        }

        reset_rpg_state(sx, sy, shp);
        g_rpg.hero_level = lv;
        g_rpg.hero_exp = (uint32_t)fge_param_get(PARAM_PERSIST_EXP, 0.0f);
        g_rpg.gold = (uint32_t)fge_param_get(PARAM_PERSIST_GOLD, 0.0f);
        g_rpg.monsters_slain = (uint32_t)fge_param_get(PARAM_PERSIST_SLAIN, 0.0f);
        uint32_t qs = (uint32_t)fge_param_get(PARAM_PERSIST_QUEST, 0.0f);
        g_rpg.quest_stage = (qs <= (uint32_t)QUEST_COMPLETED) ? (QuestStage)qs : QUEST_NOT_STARTED;
        g_rpg.boss_defeated = (g_rpg.quest_stage >= QUEST_BOSS_DEFEATED);

        fge_camera_set(g_rpg.cam_x, g_rpg.cam_y, g_rpg.cam_zoom);
        if (fge_entity_count() < 10u) {
            world_map_init();
        }
        hero_init(sx, sy, shp);
        npc_quest_init();
        monsters_init();
        update_dynamic_bgm();
    }

    rpg_spawn_burst(g_rpg.hero_pos.x, g_rpg.hero_pos.y, 18u, 175.0f, 0.48f, 8.0f, fge_rgba(120, 255, 225, 255));
    fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.7f, 1.2f);
}

void fge_update(float dt) {
    if (dt <= 0.0f) {
        return;
    }
    if (dt > 0.05f) {
        dt = 0.05f;
    }

    g_rpg.elapsed += dt;
    handle_ai_events();

    if (any_manual_input()) {
        g_rpg.idle_timer = 0.0f;
        g_rpg.autopilot_active = false;
    } else {
        g_rpg.idle_timer += dt;
        if (g_rpg.idle_timer >= RPG_AUTOPILOT_DELAY) {
            g_rpg.autopilot_active = true;
        }
    }

    update_autopilot_ai(dt);
    world_map_update(dt);
    hero_update(dt, !g_rpg.dialogue.active);
    npc_quest_update(dt);
    monsters_update(dt);

    update_dynamic_bgm();
    update_camera(dt);
    rpg_ui_update(dt);

    rpg_sync_persisted_params();

    /* Export RPG Telemetry Metrics */
    fge_log_metric(METRIC_HERO_LEVEL, (float)g_rpg.hero_level);
    fge_log_metric(METRIC_MONSTERS_SLAIN, (float)g_rpg.monsters_slain);
    fge_log_metric(METRIC_QUEST_STAGE, (float)g_rpg.quest_stage);
    fge_log_metric(METRIC_GOLD, (float)g_rpg.gold);
}

void fge_shutdown(void) {
    fge_bgm_stop();
    monsters_shutdown();
    npc_quest_shutdown();
    hero_shutdown();
    world_map_shutdown();
    g_rpg.initialized = false;
}
