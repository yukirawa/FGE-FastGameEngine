#include "rpg_shared.h"

#include <math.h>
#include <stdio.h>

#define MAX_MONSTERS 18u

typedef enum MonsterKind {
    MONSTER_SLIME_CYAN = 0,
    MONSTER_SLIME_LIME = 1,
    MONSTER_GOBLIN     = 2
} MonsterKind;

typedef struct Monster {
    bool active;
    MonsterKind kind;
    FgeEntity hitbox;
    FgeEntity shadow;
    FgeEntity body;
    FgeEntity head;
    FgeEntity eye_l;
    FgeEntity eye_r;
    FgeEntity weapon;
    float home_x;
    float home_y;
    float x;
    float y;
    float hp;
    float max_hp;
    float speed;
    float atk;
    float radius;
    float phase;
    float hit_flash;
    float respawn_timer;
    uint32_t last_slash_id;
    uint32_t exp_reward;
    uint32_t gold_reward;
} Monster;

typedef struct GolemBoss {
    bool alive;
    bool awakened;
    FgeEntity hitbox;
    FgeEntity shadow;
    FgeEntity torso;
    FgeEntity core;
    FgeEntity head;
    FgeEntity horn_l;
    FgeEntity horn_r;
    FgeEntity arm_l;
    FgeEntity arm_r;
    FgeEntity eye_l;
    FgeEntity eye_r;
    FgeEmitter emitter;
    float x;
    float y;
    float hp;
    float max_hp;
    float spin_angle;
    float slam_timer;
    float hit_flash;
    uint32_t last_slash_id;
} GolemBoss;

static Monster g_monsters[MAX_MONSTERS];
static GolemBoss g_golem;

static FgeEntity spawn_mon_part(float x, float y, float w, float h, float z, uint32_t tag, FgeColor col) {
    FgeEntity e = fge_spawn_sprite(x, y, w, h, col);
    fge_set_z_order(e, z);
    fge_set_tag(e, tag);
    fge_set_flags(e, FGE_FLAG_VISIBLE | FGE_FLAG_Y_SORT);
    return e;
}

static void despawn_monster_visuals(Monster *m) {
    FgeEntity list[7] = { m->hitbox, m->shadow, m->body, m->head, m->eye_l, m->eye_r, m->weapon };
    for (uint32_t i = 0; i < 7u; ++i) {
        if (fge_entity_is_alive(list[i])) {
            fge_entity_despawn(list[i]);
        }
    }
    m->hitbox = fge_entity_null();
    m->shadow = fge_entity_null();
    m->body = fge_entity_null();
    m->head = fge_entity_null();
    m->eye_l = fge_entity_null();
    m->eye_r = fge_entity_null();
    m->weapon = fge_entity_null();
}

static void spawn_monster_slot(uint32_t idx, MonsterKind kind, float hx, float hy) {
    Monster *m = &g_monsters[idx];
    despawn_monster_visuals(m);
    m->active = true;
    m->kind = kind;
    m->home_x = hx;
    m->home_y = hy;
    m->x = hx;
    m->y = hy;
    m->phase = (float)idx * 1.1f;
    m->hit_flash = 0.0f;
    m->respawn_timer = 0.0f;
    m->last_slash_id = 0u;

    uint32_t tag = (kind == MONSTER_GOBLIN) ? TAG_MONSTER_GOBLIN : TAG_MONSTER_SLIME;
    if (kind == MONSTER_GOBLIN) {
        m->max_hp = 68.0f;
        m->speed = 132.0f;
        m->atk = 15.0f;
        m->radius = 17.0f;
        m->exp_reward = 45u;
        m->gold_reward = 24u;
    } else {
        m->max_hp = 40.0f;
        m->speed = 86.0f;
        m->atk = 10.0f;
        m->radius = 15.0f;
        m->exp_reward = 25u;
        m->gold_reward = 12u;
    }
    m->hp = m->max_hp;

    m->hitbox = fge_spawn_sprite(hx, hy, m->radius * 2.0f, m->radius * 2.0f, fge_rgba(0, 0, 0, 0));
    fge_set_flags(m->hitbox, FGE_FLAG_COLLIDABLE);
    fge_set_collider(m->hitbox, m->radius, LAYER_MONSTER);
    fge_set_tag(m->hitbox, tag);
    fge_set_hp(m->hitbox, m->hp);

    m->shadow = fge_spawn_sprite(hx, hy + 11.0f, 22.0f, 9.0f, fge_rgba(8, 14, 10, 100));
    fge_set_z_order(m->shadow, -1.0f);
    fge_set_flags(m->shadow, FGE_FLAG_VISIBLE);

    if (kind == MONSTER_GOBLIN) {
        m->body = spawn_mon_part(hx, hy, 18.0f, 16.0f, 0.20f, tag, fge_rgba(142, 62, 48, 255));
        m->head = spawn_mon_part(hx, hy - 11.0f, 16.0f, 13.0f, 1.15f, tag, fge_rgba(104, 168, 66, 255));
        m->eye_l = spawn_mon_part(hx - 3.5f, hy - 10.0f, 3.0f, 3.0f, 1.30f, tag, fge_rgba(255, 224, 65, 255));
        m->eye_r = spawn_mon_part(hx + 3.5f, hy - 10.0f, 3.0f, 3.0f, 1.30f, tag, fge_rgba(255, 224, 65, 255));
        m->weapon = spawn_mon_part(hx + 11.0f, hy - 2.0f, 5.0f, 18.0f, 0.30f, tag, fge_rgba(155, 112, 68, 255));
    } else {
        FgeColor body_col = (kind == MONSTER_SLIME_CYAN)
            ? fge_rgba(52, 208, 245, 238)
            : fge_rgba(116, 232, 74, 238);
        m->body = spawn_mon_part(hx, hy, 26.0f, 20.0f, 0.20f, tag, body_col);
        m->eye_l = spawn_mon_part(hx - 5.0f, hy + 1.0f, 3.5f, 4.5f, 0.35f, tag, fge_rgba(18, 28, 42, 255));
        m->eye_r = spawn_mon_part(hx + 5.0f, hy + 1.0f, 3.5f, 4.5f, 0.35f, tag, fge_rgba(18, 28, 42, 255));
    }
}

static void spawn_golem_boss(void) {
    g_golem.alive = true;
    g_golem.awakened = false;
    g_golem.x = RPG_GOLEM_HOME_X;
    g_golem.y = RPG_GOLEM_HOME_Y;
    g_golem.max_hp = 260.0f;
    g_golem.hp = g_golem.max_hp;
    g_golem.spin_angle = 0.0f;
    g_golem.slam_timer = 3.2f;
    g_golem.hit_flash = 0.0f;
    g_golem.last_slash_id = 0u;

    g_rpg.boss_hp = g_golem.hp;
    g_rpg.boss_max_hp = g_golem.max_hp;

    g_golem.hitbox = fge_spawn_sprite(g_golem.x, g_golem.y, 64.0f, 64.0f, fge_rgba(0, 0, 0, 0));
    fge_set_flags(g_golem.hitbox, FGE_FLAG_COLLIDABLE);
    fge_set_collider(g_golem.hitbox, 34.0f, LAYER_MONSTER);
    fge_set_tag(g_golem.hitbox, TAG_BOSS_GOLEM);
    fge_set_hp(g_golem.hitbox, g_golem.hp);

    g_golem.shadow = fge_spawn_sprite(g_golem.x, g_golem.y + 30.0f, 76.0f, 26.0f, fge_rgba(10, 8, 18, 145));
    fge_set_z_order(g_golem.shadow, -1.0f);
    fge_set_flags(g_golem.shadow, FGE_FLAG_VISIBLE);

    g_golem.torso = spawn_mon_part(g_golem.x, g_golem.y, 60.0f, 54.0f, 0.20f, TAG_BOSS_GOLEM, fge_rgba(44, 40, 58, 255));
    g_golem.core = spawn_mon_part(g_golem.x, g_golem.y + 2.0f, 20.0f, 20.0f, 0.32f, TAG_BOSS_GOLEM, fge_rgba(188, 72, 255, 255));
    g_golem.arm_l = spawn_mon_part(g_golem.x - 38.0f, g_golem.y + 4.0f, 22.0f, 30.0f, 0.25f, TAG_BOSS_GOLEM, fge_rgba(58, 52, 76, 255));
    g_golem.arm_r = spawn_mon_part(g_golem.x + 38.0f, g_golem.y + 4.0f, 22.0f, 30.0f, 0.25f, TAG_BOSS_GOLEM, fge_rgba(58, 52, 76, 255));
    g_golem.head = spawn_mon_part(g_golem.x, g_golem.y - 30.0f, 36.0f, 26.0f, 1.15f, TAG_BOSS_GOLEM, fge_rgba(52, 46, 68, 255));
    g_golem.horn_l = spawn_mon_part(g_golem.x - 16.0f, g_golem.y - 44.0f, 10.0f, 14.0f, 1.10f, TAG_BOSS_GOLEM, fge_rgba(145, 75, 215, 255));
    g_golem.horn_r = spawn_mon_part(g_golem.x + 16.0f, g_golem.y - 44.0f, 10.0f, 14.0f, 1.10f, TAG_BOSS_GOLEM, fge_rgba(145, 75, 215, 255));
    g_golem.eye_l = spawn_mon_part(g_golem.x - 8.0f, g_golem.y - 28.0f, 6.0f, 5.0f, 1.32f, TAG_BOSS_GOLEM, fge_rgba(255, 65, 95, 255));
    g_golem.eye_r = spawn_mon_part(g_golem.x + 8.0f, g_golem.y - 28.0f, 6.0f, 5.0f, 1.32f, TAG_BOSS_GOLEM, fge_rgba(255, 65, 95, 255));

    /* Step 5 Rust-native bulk radial emitter for rotating rune danmaku */
    g_golem.emitter = fge_emitter_create_radial(
        g_golem.x,
        g_golem.y,
        1.65f,
        8u,
        185.0f,
        3.0f,
        8.0f,
        LAYER_ENEMY_BULLET,
        fge_rgba(205, 85, 255, 240)
    );
    fge_emitter_set_spin(g_golem.emitter, 0.0f, 1.25f);
    fge_emitter_set_active(g_golem.emitter, false);
}

static FgeEmitter emitter_null(void) {
    FgeEmitter e = { 0xFFFFFFFFu, 0u };
    return e;
}

static void despawn_golem_all(void) {
    FgeEntity list[11] = {
        g_golem.hitbox, g_golem.shadow, g_golem.torso, g_golem.core,
        g_golem.head, g_golem.horn_l, g_golem.horn_r, g_golem.arm_l, g_golem.arm_r,
        g_golem.eye_l, g_golem.eye_r
    };
    for (uint32_t i = 0; i < 11u; ++i) {
        if (fge_entity_is_alive(list[i])) {
            fge_entity_despawn(list[i]);
        }
    }
    fge_emitter_destroy(g_golem.emitter);
    g_golem.emitter = emitter_null();
    g_golem.hitbox = fge_entity_null();
    g_golem.shadow = fge_entity_null();
    g_golem.torso = fge_entity_null();
    g_golem.core = fge_entity_null();
    g_golem.head = fge_entity_null();
    g_golem.horn_l = fge_entity_null();
    g_golem.horn_r = fge_entity_null();
    g_golem.arm_l = fge_entity_null();
    g_golem.arm_r = fge_entity_null();
    g_golem.eye_l = fge_entity_null();
    g_golem.eye_r = fge_entity_null();
}

void monsters_init(void) {
    for (uint32_t i = 0; i < MAX_MONSTERS; ++i) {
        g_monsters[i].active = false;
        g_monsters[i].hitbox = fge_entity_null();
        g_monsters[i].shadow = fge_entity_null();
        g_monsters[i].body = fge_entity_null();
        g_monsters[i].head = fge_entity_null();
        g_monsters[i].eye_l = fge_entity_null();
        g_monsters[i].eye_r = fge_entity_null();
        g_monsters[i].weapon = fge_entity_null();
    }
    g_golem.emitter = emitter_null();
    g_golem.hitbox = fge_entity_null();

    /* Southern Whispering Woods Slimes (right along the north adventure trail!) */
    spawn_monster_slot(0u, MONSTER_SLIME_CYAN, 1085.0f, 1345.0f);
    spawn_monster_slot(1u, MONSTER_SLIME_LIME, 1130.0f, 1275.0f);
    spawn_monster_slot(2u, MONSTER_SLIME_CYAN, 995.0f, 1235.0f);
    spawn_monster_slot(3u, MONSTER_SLIME_LIME, 1215.0f, 1225.0f);
    spawn_monster_slot(4u, MONSTER_SLIME_CYAN, 1070.0f, 1155.0f);

    /* Northern Whispering Woods Slimes & Forest Goblins */
    spawn_monster_slot(5u, MONSTER_SLIME_LIME, 1040.0f, 920.0f);
    spawn_monster_slot(6u, MONSTER_GOBLIN, 1130.0f, 875.0f);
    spawn_monster_slot(7u, MONSTER_GOBLIN, 990.0f, 790.0f);
    spawn_monster_slot(8u, MONSTER_SLIME_CYAN, 1210.0f, 775.0f);
    spawn_monster_slot(9u, MONSTER_GOBLIN, 1095.0f, 695.0f);
    spawn_monster_slot(10u, MONSTER_GOBLIN, 960.0f, 650.0f);
    spawn_monster_slot(11u, MONSTER_GOBLIN, 1225.0f, 645.0f);

    if (!g_rpg.boss_defeated) {
        spawn_golem_boss();
    } else {
        g_golem.alive = false;
        g_rpg.boss_active = false;
    }
}

static void damage_monster_entity(FgeEntity target, float dmg, FgeVec2 from_pos, uint32_t slash_id) {
    for (uint32_t i = 0; i < MAX_MONSTERS; ++i) {
        Monster *m = &g_monsters[i];
        if (!m->active || m->hitbox.index != target.index || m->hitbox.generation != target.generation) {
            continue;
        }
        if (m->last_slash_id == slash_id) {
            return;
        }
        m->last_slash_id = slash_id;
        m->hp -= dmg;
        m->hit_flash = 0.14f;

        char dtxt[32];
        snprintf(dtxt, sizeof(dtxt), "-%u", (uint32_t)(dmg + 0.5f));
        rpg_add_popup(m->x - 8.0f, m->y - 24.0f, 16.0f, fge_rgba(255, 240, 120, 255), dtxt);
        fge_audio_play_sfx(FGE_SFX_HIT, 0.72f, 0.96f + fge_rand_f32() * 0.12f);

        float kx = m->x - from_pos.x;
        float ky = m->y - from_pos.y;
        float kd = sqrtf(kx * kx + ky * ky);
        if (kd > 0.01f) {
            m->x += (kx / kd) * 20.0f;
            m->y += (ky / kd) * 20.0f;
            world_map_resolve_movement(&m->x, &m->y, m->radius);
        }

        if (m->hp <= 0.0f) {
            FgeColor pcol = (m->kind == MONSTER_GOBLIN)
                ? fge_rgba(165, 215, 85, 255)
                : fge_rgba(95, 235, 255, 255);
            rpg_spawn_burst(m->x, m->y, 14u, 155.0f, 0.42f, 7.0f, pcol);
            fge_audio_play_sfx(FGE_SFX_PICKUP, 0.65f, 1.0f);
            g_rpg.monsters_slain++;
            hero_gain_exp_gold(m->exp_reward, m->gold_reward);
            m->active = false;
            m->respawn_timer = 11.0f;
            despawn_monster_visuals(m);
        }
        return;
    }

    if (g_golem.alive && g_golem.hitbox.index == target.index && g_golem.hitbox.generation == target.generation) {
        if (g_golem.last_slash_id == slash_id) {
            return;
        }
        g_golem.last_slash_id = slash_id;
        g_golem.awakened = true;
        g_rpg.boss_active = true;
        if (g_rpg.quest_stage < QUEST_BOSS_FIGHT) {
            g_rpg.quest_stage = QUEST_BOSS_FIGHT;
        }
        g_golem.hp -= dmg;
        g_golem.hit_flash = 0.12f;
        g_rpg.boss_hp = rpg_clamp_f32(g_golem.hp, 0.0f, g_golem.max_hp);

        char btxt[32];
        snprintf(btxt, sizeof(btxt), "-%u", (uint32_t)(dmg + 0.5f));
        rpg_add_popup(g_golem.x - 10.0f, g_golem.y - 44.0f, 18.0f, fge_rgba(255, 195, 95, 255), btxt);
        fge_audio_play_sfx(FGE_SFX_HIT, 0.78f, 0.82f);
        rpg_add_camera_shake(0.14f);

        if (g_golem.hp <= 0.0f) {
            g_golem.alive = false;
            g_rpg.boss_active = false;
            g_rpg.boss_defeated = true;
            g_rpg.quest_stage = QUEST_BOSS_DEFEATED;
            g_rpg.monsters_slain++;
            fge_emitter_set_active(g_golem.emitter, false);
            rpg_spawn_burst(g_golem.x, g_golem.y, 32u, 220.0f, 0.75f, 11.0f, fge_rgba(215, 105, 255, 255));
            rpg_spawn_burst(g_golem.x, g_golem.y, 20u, 150.0f, 0.65f, 8.0f, fge_rgba(255, 225, 110, 255));
            fge_audio_play_sfx(FGE_SFX_EXPLOSION, 0.90f, 0.9f);
            rpg_add_camera_shake(0.45f);
            rpg_add_popup(g_golem.x - 56.0f, g_golem.y - 60.0f, 22.0f, fge_rgba(255, 232, 95, 255), "黒曜のゴーレム 討伐!");
            hero_gain_exp_gold(250u, 150u);
            despawn_golem_all();
        }
    }
}

static void update_golem_boss(float dt) {
    if (!g_golem.alive) {
        return;
    }

    if (!g_golem.awakened && g_rpg.hero_pos.y < RPG_RUINS_MAX_Y - 25.0f) {
        g_golem.awakened = true;
        g_rpg.boss_active = true;
        if (g_rpg.quest_stage < QUEST_BOSS_FIGHT) {
            g_rpg.quest_stage = QUEST_BOSS_FIGHT;
        }
        fge_emitter_set_active(g_golem.emitter, true);
        rpg_add_popup(g_golem.x - 48.0f, g_golem.y - 66.0f, 19.0f, fge_rgba(220, 120, 255, 255), "黒曜のゴーレム 覚醒!");
        fge_audio_play_sfx(FGE_SFX_EXPLOSION, 0.72f, 0.78f);
        rpg_add_camera_shake(0.28f);
    }

    if (g_golem.hit_flash > 0.0f) {
        g_golem.hit_flash -= dt;
    }

    if (g_golem.awakened) {
        float dx = g_rpg.hero_pos.x - g_golem.x;
        float dy = g_rpg.hero_pos.y - g_golem.y;
        float dist = sqrtf(dx * dx + dy * dy);
        if (dist > 44.0f) {
            float spd = (g_golem.hp < g_golem.max_hp * 0.5f) ? 86.0f : 62.0f;
            g_golem.x += (dx / dist) * spd * dt;
            g_golem.y += (dy / dist) * spd * dt;
        }
        g_golem.x = rpg_clamp_f32(g_golem.x, 720.0f, 1480.0f);
        g_golem.y = rpg_clamp_f32(g_golem.y, 130.0f, 465.0f);

        g_golem.spin_angle += dt * 1.4f;
        fge_emitter_set_position(g_golem.emitter, g_golem.x, g_golem.y);
        fge_emitter_set_spin(g_golem.emitter, g_golem.spin_angle, (g_golem.hp < g_golem.max_hp * 0.5f) ? 2.1f : 1.35f);
        fge_emitter_set_active(g_golem.emitter, !g_rpg.dialogue.active);

        /* Periodic ground shockwave slam */
        g_golem.slam_timer -= dt;
        if (g_golem.slam_timer <= 0.0f && !g_rpg.dialogue.active) {
            g_golem.slam_timer = (g_golem.hp < g_golem.max_hp * 0.5f) ? 3.2f : 4.2f;
            fge_audio_play_sfx(FGE_SFX_EXPLOSION, 0.70f, 0.95f);
            rpg_add_camera_shake(0.25f);
            for (uint32_t s = 0; s < 12u; ++s) {
                float ang = ((float)s / 12.0f) * RPG_TAU;
                FgeEntity sw = fge_spawn_sprite(g_golem.x, g_golem.y, 18.0f, 18.0f, fge_rgba(255, 135, 90, 235));
                fge_set_velocity(sw, cosf(ang) * 235.0f, sinf(ang) * 235.0f);
                fge_set_z_order(sw, 0.5f);
                fge_set_tag(sw, TAG_ENEMY_BULLET);
                fge_set_collider(sw, 9.0f, LAYER_ENEMY_BULLET);
                fge_set_flags(sw, FGE_FLAG_VISIBLE | FGE_FLAG_COLLIDABLE | FGE_FLAG_KINEMATIC);
                fge_set_ttl(sw, 0.85f);
            }
        }
    }

    float arm_bob = sinf(g_rpg.elapsed * 3.8f) * 5.0f;
    float core_sz = 18.0f + sinf(g_rpg.elapsed * 6.5f) * 4.0f;
    FgeColor body_col = (g_golem.hit_flash > 0.0f) ? fge_rgba(190, 175, 225, 255) : fge_rgba(44, 40, 58, 255);
    FgeColor core_col = (g_golem.slam_timer < 0.65f && g_golem.awakened)
        ? fge_rgba(255, 110, 75, 255)
        : fge_rgba(188, 72, 255, 255);

    if (fge_entity_is_alive(g_golem.hitbox)) {
        fge_set_position(g_golem.hitbox, g_golem.x, g_golem.y);
        fge_set_hp(g_golem.hitbox, g_golem.hp);
    }
    if (fge_entity_is_alive(g_golem.shadow)) fge_set_position(g_golem.shadow, g_golem.x, g_golem.y + 30.0f);
    if (fge_entity_is_alive(g_golem.torso)) {
        fge_set_position(g_golem.torso, g_golem.x, g_golem.y);
        fge_set_color(g_golem.torso, body_col);
    }
    if (fge_entity_is_alive(g_golem.core)) {
        fge_set_position(g_golem.core, g_golem.x, g_golem.y + 2.0f);
        fge_set_size(g_golem.core, core_sz, core_sz);
        fge_set_color(g_golem.core, core_col);
    }
    if (fge_entity_is_alive(g_golem.arm_l)) fge_set_position(g_golem.arm_l, g_golem.x - 38.0f, g_golem.y + 4.0f + arm_bob);
    if (fge_entity_is_alive(g_golem.arm_r)) fge_set_position(g_golem.arm_r, g_golem.x + 38.0f, g_golem.y + 4.0f - arm_bob);
    if (fge_entity_is_alive(g_golem.head)) fge_set_position(g_golem.head, g_golem.x, g_golem.y - 30.0f);
    if (fge_entity_is_alive(g_golem.horn_l)) fge_set_position(g_golem.horn_l, g_golem.x - 16.0f, g_golem.y - 44.0f);
    if (fge_entity_is_alive(g_golem.horn_r)) fge_set_position(g_golem.horn_r, g_golem.x + 16.0f, g_golem.y - 44.0f);
    if (fge_entity_is_alive(g_golem.eye_l)) fge_set_position(g_golem.eye_l, g_golem.x - 8.0f, g_golem.y - 28.0f);
    if (fge_entity_is_alive(g_golem.eye_r)) fge_set_position(g_golem.eye_r, g_golem.x + 8.0f, g_golem.y - 28.0f);
    g_rpg.boss_hp = g_golem.hp;
}

void monsters_update(float dt) {
    /* 1. Update Slimes & Forest Goblins */
    for (uint32_t i = 0; i < MAX_MONSTERS; ++i) {
        Monster *m = &g_monsters[i];
        if (!m->active) {
            if (m->respawn_timer > 0.0f) {
                m->respawn_timer -= dt;
                if (m->respawn_timer <= 0.0f) {
                    /* Only respawn away from Hero to avoid unfair pop-in */
                    float rdx = g_rpg.hero_pos.x - m->home_x;
                    float rdy = g_rpg.hero_pos.y - m->home_y;
                    if (rdx * rdx + rdy * rdy > 380.0f * 380.0f) {
                        spawn_monster_slot(i, m->kind, m->home_x, m->home_y);
                    } else {
                        m->respawn_timer = 3.0f;
                    }
                }
            }
            continue;
        }

        m->phase += dt * (m->kind == MONSTER_GOBLIN ? 8.0f : 5.2f);
        if (m->hit_flash > 0.0f) m->hit_flash -= dt;

        float dx = g_rpg.hero_pos.x - m->x;
        float dy = g_rpg.hero_pos.y - m->y;
        float d2 = dx * dx + dy * dy;
        float aggro = (m->kind == MONSTER_GOBLIN) ? 390.0f : 330.0f;

        if (!g_rpg.dialogue.active && d2 < aggro * aggro && d2 > 4.0f) {
            float d = sqrtf(d2);
            float hop_mul = (m->kind == MONSTER_GOBLIN)
                ? 1.0f
                : (0.45f + 0.85f * fabsf(sinf(m->phase)));
            m->x += (dx / d) * m->speed * hop_mul * dt;
            m->y += (dy / d) * m->speed * hop_mul * dt;
        } else {
            /* Gentle patrol near home clearing */
            float tx = m->home_x + cosf(m->phase * 0.28f) * 38.0f;
            float ty = m->home_y + sinf(m->phase * 0.33f) * 28.0f;
            float px = tx - m->x;
            float py = ty - m->y;
            float pd = sqrtf(px * px + py * py);
            if (pd > 2.0f) {
                m->x += (px / pd) * m->speed * 0.45f * dt;
                m->y += (py / pd) * m->speed * 0.45f * dt;
            }
        }

        world_map_resolve_movement(&m->x, &m->y, m->radius);
        /* Keep forest monsters inside the woods band */
        m->y = rpg_clamp_f32(m->y, RPG_RUINS_MAX_Y + 30.0f, RPG_VILLAGE_MIN_Y - 35.0f);

        if (fge_entity_is_alive(m->hitbox)) {
            fge_set_position(m->hitbox, m->x, m->y);
        }
        if (fge_entity_is_alive(m->shadow)) {
            fge_set_position(m->shadow, m->x, m->y + 11.0f);
        }

        if (m->kind == MONSTER_GOBLIN) {
            float bob = sinf(m->phase) * 2.0f;
            FgeColor g_skin = (m->hit_flash > 0.0f) ? fge_rgba(255, 195, 195, 255) : fge_rgba(104, 168, 66, 255);
            FgeColor g_body = (m->hit_flash > 0.0f) ? fge_rgba(255, 165, 165, 255) : fge_rgba(142, 62, 48, 255);
            if (fge_entity_is_alive(m->body)) {
                fge_set_position(m->body, m->x, m->y + bob * 0.4f);
                fge_set_color(m->body, g_body);
            }
            if (fge_entity_is_alive(m->head)) {
                fge_set_position(m->head, m->x, m->y - 11.0f + bob);
                fge_set_color(m->head, g_skin);
            }
            if (fge_entity_is_alive(m->eye_l)) fge_set_position(m->eye_l, m->x - 3.5f, m->y - 10.0f + bob);
            if (fge_entity_is_alive(m->eye_r)) fge_set_position(m->eye_r, m->x + 3.5f, m->y - 10.0f + bob);
            if (fge_entity_is_alive(m->weapon)) fge_set_position(m->weapon, m->x + 11.0f, m->y - 2.0f + bob);
        } else {
            float sq = sinf(m->phase);
            float w = 26.0f - sq * 3.5f;
            float h = 20.0f + sq * 4.0f;
            float hop_y = -fabsf(sq) * 5.0f;
            FgeColor scol = (m->hit_flash > 0.0f)
                ? fge_rgba(255, 210, 210, 255)
                : ((m->kind == MONSTER_SLIME_CYAN) ? fge_rgba(52, 208, 245, 238) : fge_rgba(116, 232, 74, 238));
            if (fge_entity_is_alive(m->body)) {
                fge_set_position(m->body, m->x, m->y + hop_y);
                fge_set_size(m->body, w, h);
                fge_set_color(m->body, scol);
            }
            if (fge_entity_is_alive(m->eye_l)) fge_set_position(m->eye_l, m->x - 5.0f, m->y + 1.0f + hop_y);
            if (fge_entity_is_alive(m->eye_r)) fge_set_position(m->eye_r, m->x + 5.0f, m->y + 1.0f + hop_y);
        }
    }

    /* 2. Update Obsidian Golem Boss */
    update_golem_boss(dt);

    /* 3. Sword Slash vs Monsters */
    uint32_t slash_id = hero_current_slash_id();
    uint32_t hit_n = fge_detect_collisions(LAYER_SWORD_SLASH, LAYER_MONSTER);
    for (uint32_t i = 0; i < hit_n; ++i) {
        FgeCollisionPair p = fge_get_collision(i);
        FgeEntity mon_ent = (fge_get_tag(p.a) == TAG_SWORD_SLASH) ? p.b : p.a;
        damage_monster_entity(mon_ent, g_rpg.hero_atk, g_rpg.hero_pos, slash_id);
    }

    /* 4. Hero vs Monster Contact Damage */
    uint32_t touch_n = fge_detect_collisions(LAYER_HERO, LAYER_MONSTER);
    for (uint32_t i = 0; i < touch_n; ++i) {
        FgeCollisionPair p = fge_get_collision(i);
        FgeEntity mon_ent = (fge_get_tag(p.a) == TAG_HERO) ? p.b : p.a;
        FgeVec2 mpos = fge_get_position(mon_ent);
        uint32_t mtag = fge_get_tag(mon_ent);
        float dmg = (mtag == TAG_BOSS_GOLEM) ? 18.0f : (mtag == TAG_MONSTER_GOBLIN ? 14.0f : 10.0f);
        hero_apply_damage(dmg, mpos);
    }

    /* 5. Hero vs Boss Rune / Shockwave Bullets */
    uint32_t bul_n = fge_detect_collisions(LAYER_HERO, LAYER_ENEMY_BULLET);
    for (uint32_t i = 0; i < bul_n; ++i) {
        FgeCollisionPair p = fge_get_collision(i);
        FgeEntity bul_ent = (fge_get_tag(p.a) == TAG_HERO) ? p.b : p.a;
        if (fge_entity_is_alive(bul_ent)) {
            FgeVec2 bpos = fge_get_position(bul_ent);
            hero_apply_damage(11.0f, bpos);
            fge_entity_despawn(bul_ent);
        }
    }
}

bool monsters_find_nearest_target(float x, float y, float max_range, FgeVec2 *out_pos, bool *out_is_boss) {
    float best_d2 = max_range * max_range;
    bool found = false;
    *out_is_boss = false;

    for (uint32_t i = 0; i < MAX_MONSTERS; ++i) {
        if (!g_monsters[i].active) {
            continue;
        }
        float dx = g_monsters[i].x - x;
        float dy = g_monsters[i].y - y;
        float d2 = dx * dx + dy * dy;
        if (d2 < best_d2) {
            best_d2 = d2;
            *out_pos = fge_vec2(g_monsters[i].x, g_monsters[i].y);
            *out_is_boss = false;
            found = true;
        }
    }

    if (g_golem.alive) {
        float dx = g_golem.x - x;
        float dy = g_golem.y - y;
        float d2 = dx * dx + dy * dy;
        if (d2 < best_d2) {
            *out_pos = fge_vec2(g_golem.x, g_golem.y);
            *out_is_boss = true;
            found = true;
        }
    }
    return found;
}

FgeVec2 monsters_get_golem_pos(void) {
    return fge_vec2(g_golem.x, g_golem.y);
}

void monsters_shutdown(void) {
    for (uint32_t i = 0; i < MAX_MONSTERS; ++i) {
        g_monsters[i].active = false;
        despawn_monster_visuals(&g_monsters[i]);
    }
    g_golem.alive = false;
    despawn_golem_all();
}
