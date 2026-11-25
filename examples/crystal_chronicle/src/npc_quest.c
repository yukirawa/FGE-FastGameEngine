#include "rpg_shared.h"

#include <math.h>
#include <string.h>

#define NPC_TALK_RANGE 108.0f

static FgeEntity g_elder_shadow = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_robe = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_staff = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_staff_orb = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_head = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_beard = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_eye_l = { 0xFFFFFFFFu, 0u };
static FgeEntity g_elder_eye_r = { 0xFFFFFFFFu, 0u };

static FgeEntity g_fairy_shadow = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_wing_l = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_wing_r = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_dress = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_head = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_halo = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_eye_l = { 0xFFFFFFFFu, 0u };
static FgeEntity g_fairy_eye_r = { 0xFFFFFFFFu, 0u };

static float g_npc_anim_time = 0.0f;

static FgeEntity spawn_npc_part(float x, float y, float w, float h, float z, FgeColor col) {
    FgeEntity e = fge_spawn_sprite(x, y, w, h, col);
    fge_set_z_order(e, z);
    fge_set_tag(e, TAG_NPC);
    fge_set_flags(e, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT);
    return e;
}

static uint32_t count_utf8_chars(const char *s) {
    uint32_t chars = 0u;
    if (!s) {
        return 0u;
    }
    const unsigned char *p = (const unsigned char *)s;
    while (*p != '\0') {
        if ((*p & 0xC0u) != 0x80u) {
            chars++;
        }
        p++;
    }
    return chars;
}

static void copy_cstr(char *dst, size_t dst_cap, const char *src) {
    if (dst_cap == 0u) {
        return;
    }
    size_t i = 0u;
    if (src) {
        while (i + 1u < dst_cap && src[i] != '\0') {
            dst[i] = src[i];
            i++;
        }
    }
    dst[i] = '\0';
}

static void start_dialogue(const char *speaker, const char *p0, const char *p1, DialogAction action) {
    DialogueState *d = &g_rpg.dialogue;
    d->active = true;
    copy_cstr(d->speaker, sizeof(d->speaker), speaker);
    d->page_count = 0u;
    if (p0 && p0[0] != '\0') {
        copy_cstr(d->pages[d->page_count++], sizeof(d->pages[0]), p0);
    }
    if (p1 && p1[0] != '\0' && d->page_count < MAX_DIALOG_PAGES) {
        copy_cstr(d->pages[d->page_count++], sizeof(d->pages[0]), p1);
    }
    d->current_page = 0u;
    d->visible_chars = 0u;
    d->total_chars = (d->page_count > 0u) ? count_utf8_chars(d->pages[0]) : 0u;
    d->type_timer = 0.0f;
    d->blip_timer = 0.0f;
    d->auto_wait_timer = 0.0f;
    d->on_close_action = action;
}

void npc_quest_open_elder_dialogue(void) {
    if (g_rpg.quest_stage == QUEST_NOT_STARTED) {
        fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.72f, 1.0f);
        start_dialogue(
            "村長",
            "勇者よ！北の遺跡に『黒曜のゴーレム』が目覚めた！",
            "森の魔物を倒して鍛え、ゴーレムを討伐してくれ！",
            DIALOG_ACTION_START_QUEST
        );
    } else if (g_rpg.quest_stage == QUEST_BOSS_DEFEATED || g_rpg.quest_stage == QUEST_COMPLETED) {
        fge_audio_play_sfx(FGE_SFX_LEVEL_UP, 0.85f, 1.0f);
        start_dialogue(
            "村長",
            "見事じゃ勇者よ！黒曜のゴーレムは倒れ、村に平和が戻ったぞ！ありがとう！",
            NULL,
            DIALOG_ACTION_COMPLETE_QUEST
        );
    } else {
        fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.65f, 1.0f);
        start_dialogue(
            "村長",
            "森で鍛えて北の遺跡へ向かうのじゃ！橋の近くに妖精もおるぞ！",
            NULL,
            DIALOG_ACTION_NONE
        );
    }
}

void npc_quest_open_fairy_dialogue(void) {
    hero_heal_full();
    fge_audio_play_sfx(FGE_SFX_HEAL, 0.85f, 1.0f);
    start_dialogue(
        "森の妖精",
        "光の祝福を授けます！HPが全回復しました！北の祭壇に気をつけて…！",
        NULL,
        DIALOG_ACTION_FAIRY_HEAL
    );
}

static void finish_dialogue(void) {
    DialogueState *d = &g_rpg.dialogue;
    DialogAction act = d->on_close_action;
    d->active = false;
    d->on_close_action = DIALOG_ACTION_NONE;

    if (act == DIALOG_ACTION_START_QUEST && g_rpg.quest_stage == QUEST_NOT_STARTED) {
        g_rpg.quest_stage = QUEST_ACTIVE;
        fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.75f, 1.15f);
        rpg_add_popup(g_rpg.hero_pos.x - 44.0f, g_rpg.hero_pos.y - 42.0f, 17.0f, fge_rgba(255, 232, 115, 255), "クエスト受注!");
    } else if (act == DIALOG_ACTION_COMPLETE_QUEST && g_rpg.quest_stage == QUEST_BOSS_DEFEATED) {
        g_rpg.quest_stage = QUEST_COMPLETED;
        g_rpg.gold += 200u;
        rpg_spawn_burst(RPG_ELDER_X, RPG_ELDER_Y, 24u, 160.0f, 0.65f, 8.0f, fge_rgba(255, 230, 95, 255));
        rpg_add_popup(RPG_ELDER_X - 42.0f, RPG_ELDER_Y - 52.0f, 18.0f, fge_rgba(255, 236, 110, 255), "クエスト達成!");
    }
}

void npc_quest_advance_dialogue(void) {
    DialogueState *d = &g_rpg.dialogue;
    if (!d->active) {
        return;
    }
    if (d->visible_chars < d->total_chars) {
        d->visible_chars = d->total_chars;
        d->auto_wait_timer = 0.0f;
        fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.45f, 1.1f);
        return;
    }
    if (d->current_page + 1u < d->page_count) {
        d->current_page++;
        d->visible_chars = 0u;
        d->total_chars = count_utf8_chars(d->pages[d->current_page]);
        d->type_timer = 0.0f;
        d->auto_wait_timer = 0.0f;
        fge_audio_play_sfx(FGE_SFX_CONFIRM, 0.55f, 1.05f);
    } else {
        finish_dialogue();
    }
}

static bool is_near_point(float x, float y, float range) {
    float dx = g_rpg.hero_pos.x - x;
    float dy = g_rpg.hero_pos.y - y;
    return (dx * dx + dy * dy) <= (range * range);
}

bool npc_quest_has_nearby_npc(void) {
    return is_near_point(RPG_ELDER_X, RPG_ELDER_Y, NPC_TALK_RANGE)
        || is_near_point(RPG_FAIRY_X, RPG_FAIRY_Y, NPC_TALK_RANGE);
}

bool npc_quest_try_interact(void) {
    if (g_rpg.dialogue.active) {
        npc_quest_advance_dialogue();
        return true;
    }
    if (is_near_point(RPG_ELDER_X, RPG_ELDER_Y, NPC_TALK_RANGE)) {
        npc_quest_open_elder_dialogue();
        return true;
    }
    if (is_near_point(RPG_FAIRY_X, RPG_FAIRY_Y, NPC_TALK_RANGE)) {
        npc_quest_open_fairy_dialogue();
        return true;
    }
    return false;
}

void npc_quest_init(void) {
    g_npc_anim_time = 0.0f;
    memset(&g_rpg.dialogue, 0, sizeof(g_rpg.dialogue));

    /* Village Elder (村長) */
    g_elder_shadow = fge_spawn_sprite(RPG_ELDER_X, RPG_ELDER_Y + 13.0f, 24.0f, 10.0f, fge_rgba(12, 16, 12, 105));
    fge_set_z_order(g_elder_shadow, -1.0f);
    fge_set_flags(g_elder_shadow, FGE_FLAG_VISIBLE);

    g_elder_robe = spawn_npc_part(RPG_ELDER_X, RPG_ELDER_Y, 20.0f, 20.0f, 0.20f, fge_rgba(68, 72, 152, 255));
    g_elder_staff = spawn_npc_part(RPG_ELDER_X + 13.0f, RPG_ELDER_Y - 4.0f, 4.0f, 28.0f, 0.25f, fge_rgba(138, 92, 46, 255));
    g_elder_staff_orb = spawn_npc_part(RPG_ELDER_X + 13.0f, RPG_ELDER_Y - 19.0f, 8.0f, 8.0f, 1.35f, fge_rgba(110, 232, 255, 255));
    g_elder_head = spawn_npc_part(RPG_ELDER_X, RPG_ELDER_Y - 12.0f, 18.0f, 14.0f, 1.15f, fge_rgba(244, 208, 178, 255));
    g_elder_beard = spawn_npc_part(RPG_ELDER_X, RPG_ELDER_Y - 5.0f, 14.0f, 9.0f, 1.25f, fge_rgba(238, 240, 246, 255));
    g_elder_eye_l = spawn_npc_part(RPG_ELDER_X - 4.0f, RPG_ELDER_Y - 10.0f, 3.0f, 3.0f, 1.32f, fge_rgba(28, 32, 48, 255));
    g_elder_eye_r = spawn_npc_part(RPG_ELDER_X + 4.0f, RPG_ELDER_Y - 10.0f, 3.0f, 3.0f, 1.32f, fge_rgba(28, 32, 48, 255));

    /* Forest Fairy (森の妖精) near the bridge */
    g_fairy_shadow = fge_spawn_sprite(RPG_FAIRY_X, RPG_FAIRY_Y + 14.0f, 18.0f, 8.0f, fge_rgba(12, 22, 18, 90));
    fge_set_z_order(g_fairy_shadow, -1.0f);
    fge_set_flags(g_fairy_shadow, FGE_FLAG_VISIBLE);

    g_fairy_wing_l = spawn_npc_part(RPG_FAIRY_X - 10.0f, RPG_FAIRY_Y - 6.0f, 12.0f, 14.0f, 0.10f, fge_rgba(170, 250, 240, 200));
    g_fairy_wing_r = spawn_npc_part(RPG_FAIRY_X + 10.0f, RPG_FAIRY_Y - 6.0f, 12.0f, 14.0f, 0.10f, fge_rgba(170, 250, 240, 200));
    g_fairy_dress = spawn_npc_part(RPG_FAIRY_X, RPG_FAIRY_Y, 16.0f, 16.0f, 0.22f, fge_rgba(78, 225, 165, 255));
    g_fairy_head = spawn_npc_part(RPG_FAIRY_X, RPG_FAIRY_Y - 12.0f, 15.0f, 13.0f, 1.15f, fge_rgba(255, 226, 205, 255));
    g_fairy_halo = spawn_npc_part(RPG_FAIRY_X, RPG_FAIRY_Y - 21.0f, 14.0f, 4.0f, 1.25f, fge_rgba(255, 238, 125, 245));
    g_fairy_eye_l = spawn_npc_part(RPG_FAIRY_X - 3.5f, RPG_FAIRY_Y - 10.5f, 2.5f, 3.5f, 1.32f, fge_rgba(32, 95, 135, 255));
    g_fairy_eye_r = spawn_npc_part(RPG_FAIRY_X + 3.5f, RPG_FAIRY_Y - 10.5f, 2.5f, 3.5f, 1.32f, fge_rgba(32, 95, 135, 255));
}

void npc_quest_update(float dt) {
    g_npc_anim_time += dt;

    /* Subtle NPC idle & fairy wing flutter */
    float orb_pulse = 7.0f + sinf(g_npc_anim_time * 4.0f) * 2.0f;
    if (fge_entity_is_alive(g_elder_staff_orb)) {
        fge_set_size(g_elder_staff_orb, orb_pulse, orb_pulse);
    }

    float f_bob = sinf(g_npc_anim_time * 3.4f) * 4.0f;
    float wing_w = 10.0f + fabsf(sinf(g_npc_anim_time * 14.0f)) * 6.0f;
    if (fge_entity_is_alive(g_fairy_dress)) {
        fge_set_position(g_fairy_dress, RPG_FAIRY_X, RPG_FAIRY_Y + f_bob);
    }
    if (fge_entity_is_alive(g_fairy_wing_l)) {
        fge_set_position(g_fairy_wing_l, RPG_FAIRY_X - 10.0f, RPG_FAIRY_Y - 5.0f + f_bob);
        fge_set_size(g_fairy_wing_l, wing_w, 13.0f);
    }
    if (fge_entity_is_alive(g_fairy_wing_r)) {
        fge_set_position(g_fairy_wing_r, RPG_FAIRY_X + 10.0f, RPG_FAIRY_Y - 5.0f + f_bob);
        fge_set_size(g_fairy_wing_r, wing_w, 13.0f);
    }
    if (fge_entity_is_alive(g_fairy_head)) {
        fge_set_position(g_fairy_head, RPG_FAIRY_X, RPG_FAIRY_Y - 12.0f + f_bob);
    }
    if (fge_entity_is_alive(g_fairy_halo)) {
        fge_set_position(g_fairy_halo, RPG_FAIRY_X, RPG_FAIRY_Y - 21.0f + f_bob);
    }
    if (fge_entity_is_alive(g_fairy_eye_l)) {
        fge_set_position(g_fairy_eye_l, RPG_FAIRY_X - 3.5f, RPG_FAIRY_Y - 10.5f + f_bob);
    }
    if (fge_entity_is_alive(g_fairy_eye_r)) {
        fge_set_position(g_fairy_eye_r, RPG_FAIRY_X + 3.5f, RPG_FAIRY_Y - 10.5f + f_bob);
    }

    /* Overhead quest marker & proximity prompts */
    if (!g_rpg.dialogue.active) {
        if (g_rpg.quest_stage == QUEST_NOT_STARTED || g_rpg.quest_stage == QUEST_BOSS_DEFEATED) {
            fge_world_text(RPG_ELDER_X - 6.0f, RPG_ELDER_Y - 40.0f, 18.0f, fge_rgba(255, 230, 85, 255), fge_str("!"));
        }
        if (is_near_point(RPG_ELDER_X, RPG_ELDER_Y, NPC_TALK_RANGE)) {
            fge_world_text(
                RPG_ELDER_X - 46.0f, RPG_ELDER_Y - 62.0f, 15.0f,
                fge_rgba(255, 245, 170, 255), fge_str("[SPACE] 話す")
            );
        } else if (is_near_point(RPG_FAIRY_X, RPG_FAIRY_Y, NPC_TALK_RANGE)) {
            fge_world_text(
                RPG_FAIRY_X - 46.0f, RPG_FAIRY_Y - 52.0f, 15.0f,
                fge_rgba(165, 255, 225, 255), fge_str("[SPACE] 話す")
            );
        }
    }

    /* Typewriter dialogue progression */
    DialogueState *d = &g_rpg.dialogue;
    if (!d->active) {
        return;
    }

    if (d->visible_chars < d->total_chars) {
        d->type_timer += dt;
        d->blip_timer -= dt;
        while (d->type_timer >= 0.028f && d->visible_chars < d->total_chars) {
            d->type_timer -= 0.028f;
            d->visible_chars++;
            if (d->blip_timer <= 0.0f) {
                d->blip_timer = 0.06f;
                fge_audio_play_sfx(FGE_SFX_BLIP, 0.36f, 0.94f + fge_rand_f32() * 0.14f);
            }
        }
    } else if (g_rpg.autopilot_active) {
        d->auto_wait_timer += dt;
        if (d->auto_wait_timer >= 0.65f) {
            npc_quest_advance_dialogue();
        }
    }

    if (!g_rpg.autopilot_active) {
        if (fge_key_pressed(FGE_KEY_SPACE) || fge_key_pressed(FGE_KEY_Z) ||
            fge_key_pressed(FGE_KEY_ENTER) || fge_key_pressed(FGE_KEY_E)) {
            npc_quest_advance_dialogue();
        }
    }
}

void npc_quest_shutdown(void) {
    FgeEntity parts[16] = {
        g_elder_shadow, g_elder_robe, g_elder_staff, g_elder_staff_orb,
        g_elder_head, g_elder_beard, g_elder_eye_l, g_elder_eye_r,
        g_fairy_shadow, g_fairy_wing_l, g_fairy_wing_r, g_fairy_dress,
        g_fairy_head, g_fairy_halo, g_fairy_eye_l, g_fairy_eye_r
    };
    for (uint32_t i = 0; i < 16u; ++i) {
        if (fge_entity_is_alive(parts[i])) {
            fge_entity_despawn(parts[i]);
        }
    }
    g_elder_shadow = fge_entity_null();
    g_elder_robe = fge_entity_null();
    g_elder_staff = fge_entity_null();
    g_elder_staff_orb = fge_entity_null();
    g_elder_head = fge_entity_null();
    g_elder_beard = fge_entity_null();
    g_elder_eye_l = fge_entity_null();
    g_elder_eye_r = fge_entity_null();
    g_fairy_shadow = fge_entity_null();
    g_fairy_wing_l = fge_entity_null();
    g_fairy_wing_r = fge_entity_null();
    g_fairy_dress = fge_entity_null();
    g_fairy_head = fge_entity_null();
    g_fairy_halo = fge_entity_null();
    g_fairy_eye_l = fge_entity_null();
    g_fairy_eye_r = fge_entity_null();
}
