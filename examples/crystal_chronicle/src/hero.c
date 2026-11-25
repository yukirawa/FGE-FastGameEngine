#include "rpg_shared.h"

#include <math.h>
#include <stdio.h>

#define HERO_COLLIDER_R   16.0f
#define HERO_BASE_SPEED   225.0f
#define HERO_DASH_SPEED   610.0f
#define HERO_SLASH_CD     0.32f
#define HERO_DASH_CD      0.95f

static FgeEntity g_hero_hitbox = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_shadow = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_cape = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_boot_l = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_boot_r = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_tunic = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_belt = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_head = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_hair = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_eye_l = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_eye_r = { 0xFFFFFFFFu, 0u };
static FgeEntity g_hero_sword = { 0xFFFFFFFFu, 0u };

static float g_walk_phase = 0.0f;
static float g_slash_cd = 0.0f;
static float g_slash_anim = 0.0f;
static uint32_t g_slash_id = 0u;
static float g_dash_cd = 0.0f;
static float g_dash_timer = 0.0f;
static FgeVec2 g_dash_dir = { 0.0f, -1.0f };
static float g_afterimage_timer = 0.0f;
static float g_invuln_timer = 0.0f;
static float g_hit_flash = 0.0f;
static float g_heal_tick = 0.0f;
static FgeVec2 g_ai_move_intent = { 0.0f, 0.0f };

static FgeEntity spawn_part(float w, float h, float z, FgeColor col) {
    FgeEntity e = fge_spawn_sprite(g_rpg.hero_pos.x, g_rpg.hero_pos.y, w, h, col);
    fge_set_z_order(e, z);
    fge_set_tag(e, TAG_HERO);
    fge_set_flags(e, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT);
    return e;
}

static uint32_t exp_required_for_level(uint32_t level) {
    return 40u + level * 35u;
}

void hero_init(float start_x, float start_y, float start_hp) {
    g_rpg.hero_pos = fge_vec2(start_x, start_y);
    g_rpg.hero_dir = fge_vec2(0.0f, -1.0f);
    g_rpg.hero_max_hp = 100.0f + (float)(g_rpg.hero_level - 1u) * 30.0f;
    g_rpg.hero_hp = rpg_clamp_f32(start_hp, 1.0f, g_rpg.hero_max_hp);
    g_rpg.hero_atk = 24.0f + (float)(g_rpg.hero_level - 1u) * 9.0f;
    g_rpg.hero_exp_next = exp_required_for_level(g_rpg.hero_level);

    g_walk_phase = 0.0f;
    g_slash_cd = 0.0f;
    g_slash_anim = 0.0f;
    g_slash_id = 0u;
    g_dash_cd = 0.0f;
    g_dash_timer = 0.0f;
    g_invuln_timer = 0.4f;
    g_hit_flash = 0.0f;
    g_heal_tick = 0.0f;
    g_ai_move_intent = fge_vec2(0.0f, 0.0f);

    /* Invisible collider root entity for FGE spatial collision detection */
    g_hero_hitbox = fge_spawn_sprite(start_x, start_y, 20.0f, 20.0f, fge_rgba(0, 0, 0, 0));
    fge_set_flags(g_hero_hitbox, FGE_FLAG_COLLIDABLE);
    fge_set_collider(g_hero_hitbox, HERO_COLLIDER_R, LAYER_HERO);
    fge_set_tag(g_hero_hitbox, TAG_HERO);
    fge_set_hp(g_hero_hitbox, g_rpg.hero_hp);

    g_hero_shadow = fge_spawn_sprite(start_x, start_y + 13.0f, 24.0f, 10.0f, fge_rgba(10, 14, 12, 110));
    fge_set_z_order(g_hero_shadow, -1.0f);
    fge_set_flags(g_hero_shadow, FGE_FLAG_VISIBLE);

    g_hero_cape = spawn_part(20.0f, 18.0f, 0.08f, fge_rgba(182, 42, 54, 255));
    g_hero_boot_l = spawn_part(6.0f, 7.0f, 0.12f, fge_rgba(74, 48, 32, 255));
    g_hero_boot_r = spawn_part(6.0f, 7.0f, 0.12f, fge_rgba(74, 48, 32, 255));
    g_hero_tunic = spawn_part(18.0f, 16.0f, 0.20f, fge_rgba(38, 168, 92, 255));
    g_hero_belt = spawn_part(18.0f, 4.0f, 0.25f, fge_rgba(225, 185, 68, 255));
    g_hero_sword = spawn_part(6.0f, 18.0f, 0.30f, fge_rgba(205, 228, 245, 255));

    /* Head & eyes in layer 1 (with eyes slightly lower Y than head so eyes draw on top!) */
    g_hero_hair = spawn_part(20.0f, 8.0f, 1.08f, fge_rgba(232, 188, 72, 255));
    g_hero_head = spawn_part(18.0f, 14.0f, 1.15f, fge_rgba(248, 212, 178, 255));
    g_hero_eye_l = spawn_part(3.0f, 4.0f, 1.30f, fge_rgba(24, 34, 52, 255));
    g_hero_eye_r = spawn_part(3.0f, 4.0f, 1.30f, fge_rgba(24, 34, 52, 255));
}

void hero_set_move_intent(float dx, float dy) {
    g_ai_move_intent.x = dx;
    g_ai_move_intent.y = dy;
}

void hero_trigger_slash(void) {
    if (g_slash_cd > 0.0f) {
        return;
    }
    g_slash_cd = HERO_SLASH_CD;
    g_slash_anim = 0.18f;
    g_slash_id++;

    float pitch = 0.94f + fge_rand_f32() * 0.14f;
    fge_audio_play_sfx(FGE_SFX_SLASH, 0.72f, pitch);

    float base_ang = atan2f(g_rpg.hero_dir.y, g_rpg.hero_dir.x);
    /* Spawn 4 glowing slash arc segments with native Rust fge_set_ttl auto-despawn */
    for (uint32_t i = 0; i < 4u; ++i) {
        float t = ((float)i / 3.0f) - 0.5f;
        float ang = base_ang + t * 1.35f;
        float dist = 34.0f + fabsf(t) * 6.0f;
        float sx = g_rpg.hero_pos.x + cosf(ang) * dist;
        float sy = g_rpg.hero_pos.y + sinf(ang) * dist;

        FgeColor scol = (i == 1u || i == 2u)
            ? fge_rgba(195, 248, 255, 245)
            : fge_rgba(95, 222, 255, 215);
        FgeEntity arc = fge_spawn_sprite(sx, sy, 22.0f, 10.0f, scol);
        fge_set_rotation(arc, ang + RPG_PI * 0.5f);
        fge_set_angular_velocity(arc, (t >= 0.0f ? 4.5f : -4.5f));
        fge_set_velocity(arc, cosf(ang) * 75.0f, sinf(ang) * 75.0f);
        fge_set_z_order(arc, 1.85f);
        fge_set_tag(arc, TAG_SWORD_SLASH);
        fge_set_collider(arc, 24.0f, LAYER_SWORD_SLASH);
        fge_set_flags(arc, FGE_FLAG_VISIBLE | FGE_FLAG_COLLIDABLE | FGE_FLAG_KINEMATIC);
        fge_set_ttl(arc, 0.16f);
    }
}

void hero_trigger_dash(void) {
    if (g_dash_cd > 0.0f || g_dash_timer > 0.0f) {
        return;
    }
    g_dash_cd = HERO_DASH_CD;
    g_dash_timer = 0.17f;
    g_invuln_timer = (g_invuln_timer < 0.22f) ? 0.22f : g_invuln_timer;
    g_dash_dir = g_rpg.hero_dir;
    float len = sqrtf(g_dash_dir.x * g_dash_dir.x + g_dash_dir.y * g_dash_dir.y);
    if (len > 0.001f) {
        g_dash_dir.x /= len;
        g_dash_dir.y /= len;
    } else {
        g_dash_dir = fge_vec2(0.0f, -1.0f);
    }
    fge_audio_play_tone(FGE_WAVE_TRIANGLE, 280.0f, 620.0f, 0.11f, 0.45f);
}

void hero_apply_damage(float dmg, FgeVec2 knock_from) {
    if (g_invuln_timer > 0.0f || dmg <= 0.0f) {
        return;
    }
    g_invuln_timer = 0.72f;
    g_hit_flash = 0.18f;
    g_rpg.hero_hp -= dmg;

    char dmg_txt[32];
    snprintf(dmg_txt, sizeof(dmg_txt), "-%u", (uint32_t)(dmg + 0.5f));
    rpg_add_popup(g_rpg.hero_pos.x, g_rpg.hero_pos.y - 26.0f, 16.0f, fge_rgba(255, 95, 95, 255), dmg_txt);

    fge_audio_play_sfx(FGE_SFX_HIT, 0.65f, 0.85f);
    rpg_add_camera_shake(0.24f);

    float kx = g_rpg.hero_pos.x - knock_from.x;
    float ky = g_rpg.hero_pos.y - knock_from.y;
    float kd = sqrtf(kx * kx + ky * ky);
    if (kd > 0.01f) {
        g_rpg.hero_pos.x += (kx / kd) * 18.0f;
        g_rpg.hero_pos.y += (ky / kd) * 18.0f;
        world_map_resolve_movement(&g_rpg.hero_pos.x, &g_rpg.hero_pos.y, HERO_COLLIDER_R);
    }

    if (g_rpg.hero_hp <= 0.0f) {
        /* Hero revives safely at the Sanctuary Crystal without hard-locking gameplay */
        g_rpg.hero_hp = g_rpg.hero_max_hp;
        g_rpg.hero_pos = fge_vec2(RPG_HERO_SPAWN_X, RPG_HERO_SPAWN_Y);
        g_rpg.boss_active = false;
        if (g_rpg.quest_stage == QUEST_BOSS_FIGHT) {
            g_rpg.quest_stage = QUEST_ACTIVE;
        }
        rpg_add_popup(g_rpg.hero_pos.x - 36.0f, g_rpg.hero_pos.y - 36.0f, 18.0f, fge_rgba(120, 240, 255, 255), "聖域へ帰還!");
        fge_audio_play_sfx(FGE_SFX_HEAL, 0.80f, 1.0f);
    }
    if (fge_entity_is_alive(g_hero_hitbox)) {
        fge_set_hp(g_hero_hitbox, g_rpg.hero_hp);
    }
}

void hero_heal_full(void) {
    g_rpg.hero_hp = g_rpg.hero_max_hp;
    if (fge_entity_is_alive(g_hero_hitbox)) {
        fge_set_hp(g_hero_hitbox, g_rpg.hero_hp);
    }
    rpg_spawn_burst(g_rpg.hero_pos.x, g_rpg.hero_pos.y, 14u, 110.0f, 0.45f, 7.0f, fge_rgba(115, 255, 195, 255));
    rpg_add_popup(g_rpg.hero_pos.x - 24.0f, g_rpg.hero_pos.y - 34.0f, 17.0f, fge_rgba(130, 255, 185, 255), "HP全回復!");
}

void hero_gain_exp_gold(uint32_t exp_amt, uint32_t gold_amt) {
    g_rpg.gold += gold_amt;
    g_rpg.hero_exp += exp_amt;

    char exp_txt[48];
    snprintf(exp_txt, sizeof(exp_txt), "EXP +%u", exp_amt);
    rpg_add_popup(g_rpg.hero_pos.x - 16.0f, g_rpg.hero_pos.y - 38.0f, 15.0f, fge_rgba(110, 235, 255, 255), exp_txt);

    while (g_rpg.hero_level < RPG_MAX_LEVEL && g_rpg.hero_exp >= g_rpg.hero_exp_next) {
        g_rpg.hero_exp -= g_rpg.hero_exp_next;
        g_rpg.hero_level++;
        g_rpg.hero_exp_next = exp_required_for_level(g_rpg.hero_level);
        g_rpg.hero_max_hp = 100.0f + (float)(g_rpg.hero_level - 1u) * 30.0f;
        g_rpg.hero_hp = g_rpg.hero_max_hp;
        g_rpg.hero_atk = 24.0f + (float)(g_rpg.hero_level - 1u) * 9.0f;

        fge_audio_play_sfx(FGE_SFX_LEVEL_UP, 0.85f, 1.0f);
        rpg_spawn_burst(g_rpg.hero_pos.x, g_rpg.hero_pos.y, 20u, 165.0f, 0.55f, 8.0f, fge_rgba(255, 230, 95, 255));
        rpg_add_popup(g_rpg.hero_pos.x - 32.0f, g_rpg.hero_pos.y - 54.0f, 20.0f, fge_rgba(255, 232, 90, 255), "LEVEL UP!");
    }
}

static void sync_hero_visuals(float dt) {
    float hx = g_rpg.hero_pos.x;
    float hy = g_rpg.hero_pos.y;
    float bob = sinf(g_walk_phase * 2.0f) * 1.4f;
    float stride = sinf(g_walk_phase) * 3.2f;
    bool facing_north = (g_rpg.hero_dir.y < -0.45f);

    FgeColor tunic_col = (g_hit_flash > 0.0f)
        ? fge_rgba(255, 165, 165, 255)
        : fge_rgba(38, 168, 92, 255);
    FgeColor skin_col = (g_hit_flash > 0.0f)
        ? fge_rgba(255, 195, 195, 255)
        : fge_rgba(248, 212, 178, 255);

    if (fge_entity_is_alive(g_hero_hitbox)) {
        fge_set_position(g_hero_hitbox, hx, hy);
        fge_set_hp(g_hero_hitbox, g_rpg.hero_hp);
    }
    if (fge_entity_is_alive(g_hero_shadow)) {
        fge_set_position(g_hero_shadow, hx, hy + 13.0f);
    }

    /* Cape sits behind tunic when facing south, over back when facing north */
    if (fge_entity_is_alive(g_hero_cape)) {
        float cape_y = facing_north ? (hy + 3.0f) : (hy - 1.0f);
        float cape_z = facing_north ? 0.28f : 0.08f;
        fge_set_position(g_hero_cape, hx - g_rpg.hero_dir.x * 2.0f, cape_y + bob * 0.4f);
        fge_set_z_order(g_hero_cape, cape_z);
        fge_set_rotation(g_hero_cape, sinf(g_walk_phase) * 0.08f);
    }

    if (fge_entity_is_alive(g_hero_boot_l)) {
        fge_set_position(g_hero_boot_l, hx - 5.0f, hy + 10.0f + stride);
    }
    if (fge_entity_is_alive(g_hero_boot_r)) {
        fge_set_position(g_hero_boot_r, hx + 5.0f, hy + 10.0f - stride);
    }
    if (fge_entity_is_alive(g_hero_tunic)) {
        fge_set_position(g_hero_tunic, hx, hy + bob * 0.5f);
        fge_set_color(g_hero_tunic, tunic_col);
    }
    if (fge_entity_is_alive(g_hero_belt)) {
        fge_set_position(g_hero_belt, hx, hy + 4.0f + bob * 0.5f);
    }

    /* Head & hair in layer 1 */
    float head_y = hy - 11.0f + bob;
    if (fge_entity_is_alive(g_hero_head)) {
        fge_set_position(g_hero_head, hx, head_y);
        fge_set_color(g_hero_head, skin_col);
    }
    if (fge_entity_is_alive(g_hero_hair)) {
        fge_set_position(g_hero_hair, hx, head_y - 6.0f);
    }

    /* Directional eyes (y = head_y + 1.5 > head_y so Y-sort places eyes cleanly in front of face) */
    uint32_t eye_vis = facing_north ? 0u : (FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT);
    float ex_off = g_rpg.hero_dir.x * 2.8f;
    float ey_off = g_rpg.hero_dir.y * 1.2f;
    if (fge_entity_is_alive(g_hero_eye_l)) {
        fge_set_flags(g_hero_eye_l, eye_vis);
        fge_set_position(g_hero_eye_l, hx - 4.0f + ex_off, head_y + 1.5f + ey_off);
    }
    if (fge_entity_is_alive(g_hero_eye_r)) {
        fge_set_flags(g_hero_eye_r, eye_vis);
        fge_set_position(g_hero_eye_r, hx + 4.0f + ex_off, head_y + 1.5f + ey_off);
    }

    /* Equipped sword blade swings cleanly during g_slash_anim */
    if (fge_entity_is_alive(g_hero_sword)) {
        float aim = atan2f(g_rpg.hero_dir.y, g_rpg.hero_dir.x);
        float swing_offset = (g_slash_anim > 0.0f) ? ((g_slash_anim / 0.18f - 0.5f) * 2.2f) : 0.7f;
        float s_ang = aim + swing_offset;
        float s_dist = (g_slash_anim > 0.0f) ? 20.0f : 14.0f;
        fge_set_position(g_hero_sword, hx + cosf(s_ang) * s_dist, hy + sinf(s_ang) * s_dist);
        fge_set_rotation(g_hero_sword, s_ang + RPG_PI * 0.5f);
        fge_set_z_order(g_hero_sword, facing_north ? 0.10f : 0.32f);
    }
    (void)dt;
}

void hero_update(float dt, bool allow_control) {
    if (g_slash_cd > 0.0f) g_slash_cd -= dt;
    if (g_slash_anim > 0.0f) g_slash_anim -= dt;
    if (g_dash_cd > 0.0f) g_dash_cd -= dt;
    if (g_invuln_timer > 0.0f) g_invuln_timer -= dt;
    if (g_hit_flash > 0.0f) g_hit_flash -= dt;

    /* Sanctuary Crystal gentle healing aura in Village Square */
    float cdx = g_rpg.hero_pos.x - RPG_CRYSTAL_X;
    float cdy = g_rpg.hero_pos.y - RPG_CRYSTAL_Y;
    if (cdx * cdx + cdy * cdy < 95.0f * 95.0f && g_rpg.hero_hp < g_rpg.hero_max_hp) {
        g_heal_tick += dt;
        g_rpg.hero_hp = rpg_clamp_f32(g_rpg.hero_hp + 12.0f * dt, 0.0f, g_rpg.hero_max_hp);
        if (g_heal_tick >= 0.9f) {
            g_heal_tick = 0.0f;
            rpg_add_popup(g_rpg.hero_pos.x, g_rpg.hero_pos.y - 28.0f, 14.0f, fge_rgba(110, 255, 220, 255), "+HP");
        }
    }

    float move_x = 0.0f;
    float move_y = 0.0f;

    if (allow_control) {
        if (!g_rpg.autopilot_active) {
            if (fge_key_down(FGE_KEY_A) || fge_key_down(FGE_KEY_LEFT)) move_x -= 1.0f;
            if (fge_key_down(FGE_KEY_D) || fge_key_down(FGE_KEY_RIGHT)) move_x += 1.0f;
            if (fge_key_down(FGE_KEY_W) || fge_key_down(FGE_KEY_UP)) move_y -= 1.0f;
            if (fge_key_down(FGE_KEY_S) || fge_key_down(FGE_KEY_DOWN)) move_y += 1.0f;

            if (fge_key_pressed(FGE_KEY_LSHIFT) || fge_key_pressed(FGE_KEY_X)) {
                hero_trigger_dash();
            }
            if (fge_key_pressed(FGE_KEY_E) || fge_key_pressed(FGE_KEY_ENTER)) {
                npc_quest_try_interact();
            } else if (fge_key_pressed(FGE_KEY_SPACE) || fge_key_pressed(FGE_KEY_Z) || fge_mouse_down(FGE_MOUSE_LEFT)) {
                if (fge_key_pressed(FGE_KEY_SPACE) && npc_quest_has_nearby_npc()) {
                    npc_quest_try_interact();
                } else {
                    hero_trigger_slash();
                }
            }
        } else {
            move_x = g_ai_move_intent.x;
            move_y = g_ai_move_intent.y;
        }
    }

    if (g_dash_timer > 0.0f) {
        g_dash_timer -= dt;
        g_rpg.hero_pos.x += g_dash_dir.x * HERO_DASH_SPEED * dt;
        g_rpg.hero_pos.y += g_dash_dir.y * HERO_DASH_SPEED * dt;
        g_afterimage_timer -= dt;
        if (g_afterimage_timer <= 0.0f) {
            g_afterimage_timer = 0.04f;
            FgeEntity ghost = fge_spawn_sprite(g_rpg.hero_pos.x, g_rpg.hero_pos.y, 18.0f, 24.0f, fge_rgba(90, 235, 195, 150));
            fge_set_z_order(ghost, 0.02f);
            fge_set_flags(ghost, FGE_FLAG_VISIBLE);
            fge_set_ttl(ghost, 0.20f);
        }
    } else {
        float mlen = sqrtf(move_x * move_x + move_y * move_y);
        if (mlen > 0.01f) {
            move_x /= mlen;
            move_y /= mlen;
            g_rpg.hero_dir = fge_vec2(move_x, move_y);
            g_rpg.hero_pos.x += move_x * HERO_BASE_SPEED * dt;
            g_rpg.hero_pos.y += move_y * HERO_BASE_SPEED * dt;
            g_walk_phase += dt * 13.0f;
        } else {
            g_walk_phase = 0.0f;
        }
    }

    world_map_resolve_movement(&g_rpg.hero_pos.x, &g_rpg.hero_pos.y, HERO_COLLIDER_R);
    sync_hero_visuals(dt);
}

uint32_t hero_current_slash_id(void) {
    return g_slash_id;
}

FgeEntity hero_get_entity(void) {
    return g_hero_hitbox;
}

void hero_shutdown(void) {
    FgeEntity parts[12] = {
        g_hero_hitbox, g_hero_shadow, g_hero_cape, g_hero_boot_l, g_hero_boot_r,
        g_hero_tunic, g_hero_belt, g_hero_head, g_hero_hair, g_hero_eye_l, g_hero_eye_r, g_hero_sword
    };
    for (uint32_t i = 0; i < 12u; ++i) {
        if (fge_entity_is_alive(parts[i])) {
            fge_entity_despawn(parts[i]);
        }
    }
    g_hero_hitbox = fge_entity_null();
    g_hero_shadow = fge_entity_null();
    g_hero_cape = fge_entity_null();
    g_hero_boot_l = fge_entity_null();
    g_hero_boot_r = fge_entity_null();
    g_hero_tunic = fge_entity_null();
    g_hero_belt = fge_entity_null();
    g_hero_head = fge_entity_null();
    g_hero_hair = fge_entity_null();
    g_hero_eye_l = fge_entity_null();
    g_hero_eye_r = fge_entity_null();
    g_hero_sword = fge_entity_null();
}
