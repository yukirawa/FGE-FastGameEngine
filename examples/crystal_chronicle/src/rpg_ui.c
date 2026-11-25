#include "rpg_shared.h"

#include <stdio.h>

void rpg_ui_update(float dt) {
    /* 1. Floating World Damage / EXP / LEVEL UP Popups */
    for (uint32_t i = 0; i < MAX_WORLD_POPUPS; ++i) {
        WorldPopup *p = &g_rpg.popups[i];
        if (!p->active) {
            continue;
        }
        p->timer += dt;
        p->y += p->vy * dt;
        if (p->timer >= p->duration) {
            p->active = false;
            continue;
        }
        float alpha_frac = 1.0f - (p->timer / p->duration);
        uint8_t a = (uint8_t)(rpg_clamp_f32(alpha_frac * 1.4f, 0.0f, 1.0f) * (float)p->color.a);
        FgeColor col = fge_rgba(p->color.r, p->color.g, p->color.b, a);
        fge_world_text(p->x, p->y, p->font_size, col, fge_str(p->text));
    }

    /* 2. Top-Left Classic RPG Status Panel */
    fge_ui_panel(
        16.0f, 16.0f, 412.0f, 146.0f,
        fge_rgba(14, 18, 34, 215), fge_rgba(186, 162, 92, 245), 2.5f
    );

    char lv_buf[120];
    snprintf(lv_buf, sizeof(lv_buf), "勇者  LV.%u", g_rpg.hero_level);
    fge_ui_text(30.0f, 24.0f, 20.0f, fge_rgba(255, 232, 128, 255), fge_str(lv_buf));

    /* HP Bar */
    float hp_ratio = (g_rpg.hero_max_hp > 0.0f) ? (g_rpg.hero_hp / g_rpg.hero_max_hp) : 0.0f;
    FgeColor hp_col = (hp_ratio > 0.4f)
        ? fge_rgba(68, 224, 118, 255)
        : fge_rgba(242, 78, 78, 255);
    fge_ui_text(30.0f, 54.0f, 15.0f, fge_rgba(225, 235, 245, 255), fge_str("HP"));
    fge_ui_bar(72.0f, 54.0f, 210.0f, 16.0f, hp_ratio, hp_col, fge_rgba(28, 20, 32, 235));
    fge_ui_number(294.0f, 53.0f, 16.0f, (int64_t)(g_rpg.hero_hp + 0.5f), fge_rgba(240, 248, 255, 255));

    /* EXP Bar */
    float exp_ratio = (g_rpg.hero_level >= RPG_MAX_LEVEL)
        ? 1.0f
        : ((g_rpg.hero_exp_next > 0u) ? ((float)g_rpg.hero_exp / (float)g_rpg.hero_exp_next) : 0.0f);
    fge_ui_text(30.0f, 78.0f, 14.0f, fge_rgba(165, 225, 255, 255), fge_str("EXP"));
    fge_ui_bar(72.0f, 79.0f, 210.0f, 12.0f, exp_ratio, fge_rgba(72, 208, 255, 255), fge_rgba(18, 26, 42, 235));

    /* Quest Objective */
    const char *quest_line = "クエスト：村長に話しかけよう";
    if (g_rpg.quest_stage == QUEST_ACTIVE) {
        quest_line = "クエスト：黒曜のゴーレムを討伐せよ";
    } else if (g_rpg.quest_stage == QUEST_BOSS_FIGHT) {
        quest_line = "クエスト：黒曜のゴーレムを討伐せよ";
    } else if (g_rpg.quest_stage == QUEST_BOSS_DEFEATED) {
        quest_line = "クエスト：村長に勝利を報告しよう";
    } else if (g_rpg.quest_stage == QUEST_COMPLETED) {
        quest_line = "クエスト完了！村に平和が戻った！";
    }
    fge_ui_text(30.0f, 102.0f, 15.0f, fge_rgba(255, 244, 195, 255), fge_str(quest_line));

    char stat_buf[120];
    snprintf(stat_buf, sizeof(stat_buf), "討伐数: %u   GOLD: %u", g_rpg.monsters_slain, g_rpg.gold);
    fge_ui_text(30.0f, 128.0f, 15.0f, fge_rgba(205, 225, 245, 255), fge_str(stat_buf));

    /* 3. Top-Right Zone & Controls Badge */
    fge_ui_panel(
        960.0f, 16.0f, 304.0f, 76.0f,
        fge_rgba(14, 18, 34, 205), fge_rgba(110, 145, 195, 230), 2.0f
    );
    const char *zone_name = "ささやきの森";
    if (g_rpg.hero_pos.y >= RPG_VILLAGE_MIN_Y) {
        zone_name = "南の平和な村";
    } else if (g_rpg.hero_pos.y <= RPG_RUINS_MAX_Y) {
        zone_name = "北の古代遺跡";
    }
    char zone_buf[120];
    snprintf(zone_buf, sizeof(zone_buf), "%s  %s", zone_name, g_rpg.autopilot_active ? "[AUTO]" : "[MANUAL]");
    fge_ui_text(974.0f, 26.0f, 16.0f, fge_rgba(165, 242, 215, 255), fge_str(zone_buf));
    fge_ui_text(974.0f, 54.0f, 13.0f, fge_rgba(205, 215, 232, 235), fge_str("WASD:移動 SPACE:斬る/話す SHIFT:回避"));

    /* 4. Boss HP Bar in Northern Ruins */
    if (g_rpg.boss_active && !g_rpg.boss_defeated) {
        float b_ratio = (g_rpg.boss_max_hp > 0.0f) ? (g_rpg.boss_hp / g_rpg.boss_max_hp) : 0.0f;
        fge_ui_panel(
            446.0f, 18.0f, 480.0f, 62.0f,
            fge_rgba(22, 12, 34, 225), fge_rgba(205, 92, 255, 245), 2.5f
        );
        fge_ui_text(612.0f, 25.0f, 18.0f, fge_rgba(245, 195, 255, 255), fge_str("黒曜のゴーレム"));
        fge_ui_bar(466.0f, 52.0f, 440.0f, 16.0f, b_ratio, fge_rgba(198, 65, 255, 255), fge_rgba(32, 14, 42, 235));
    }

    /* 5. Japanese Typewriter Dialogue Box */
    if (g_rpg.dialogue.active && g_rpg.dialogue.page_count > 0u) {
        uint32_t page_idx = g_rpg.dialogue.current_page;
        if (page_idx >= g_rpg.dialogue.page_count) {
            page_idx = g_rpg.dialogue.page_count - 1u;
        }
        fge_ui_dialog_box(
            160.0f, 515.0f, 960.0f, 175.0f,
            fge_str(g_rpg.dialogue.speaker),
            fge_str(g_rpg.dialogue.pages[page_idx]),
            g_rpg.dialogue.visible_chars
        );
    }
}
