#ifndef RPG_SHARED_H
#define RPG_SHARED_H

#include <stdbool.h>
#include <stdint.h>

#include "fge.h"

/* ==========================================================================
 * World Layout & Math Constants
 * ========================================================================== */

#define RPG_VIEW_W          1280.0f
#define RPG_VIEW_H          720.0f
#define RPG_WORLD_W         2200.0f
#define RPG_WORLD_H         2000.0f
#define RPG_MARGIN          48.0f
#define RPG_PI              3.14159265358979323846f
#define RPG_TAU             6.28318530717958647692f

#define RPG_AUTOPILOT_DELAY 2.5f
#define RPG_MAX_LEVEL       5u

/* Zone Y-bands (+X right, +Y south/down) */
#define RPG_RUINS_MAX_Y     560.0f
#define RPG_RIVER_MIN_Y     1010.0f
#define RPG_RIVER_MAX_Y     1075.0f
#define RPG_BRIDGE_MIN_X    1030.0f
#define RPG_BRIDGE_MAX_X    1170.0f
#define RPG_VILLAGE_MIN_Y   1420.0f

/* Key Landmarks */
#define RPG_HERO_SPAWN_X    1100.0f
#define RPG_HERO_SPAWN_Y    1760.0f
#define RPG_CRYSTAL_X       1100.0f
#define RPG_CRYSTAL_Y       1560.0f
#define RPG_ELDER_X         1015.0f
#define RPG_ELDER_Y         1615.0f
#define RPG_FAIRY_X         1195.0f
#define RPG_FAIRY_Y         1095.0f
#define RPG_GOLEM_HOME_X    1100.0f
#define RPG_GOLEM_HOME_Y    290.0f

/* ==========================================================================
 * Collision Layer Masks & Entity Tags
 * ========================================================================== */

enum {
    LAYER_HERO         = 1u << 0,
    LAYER_MONSTER      = 1u << 1,
    LAYER_SWORD_SLASH  = 1u << 2,
    LAYER_ENEMY_BULLET = 1u << 3
};

enum {
    TAG_WORLD_GROUND   = 1u,
    TAG_WORLD_PROP     = 2u,
    TAG_HERO           = 3u,
    TAG_SWORD_SLASH    = 4u,
    TAG_NPC            = 5u,
    TAG_MONSTER_SLIME  = 6u,
    TAG_MONSTER_GOBLIN = 7u,
    TAG_BOSS_GOLEM     = 8u,
    TAG_ENEMY_BULLET   = 9u,
    TAG_FX             = 10u
};

/* ==========================================================================
 * Quest Stages, Metrics & Hot-Reload Persistence Params
 * ========================================================================== */

typedef enum QuestStage {
    QUEST_NOT_STARTED   = 0,
    QUEST_ACTIVE        = 1,
    QUEST_BOSS_FIGHT    = 2,
    QUEST_BOSS_DEFEATED = 3,
    QUEST_COMPLETED     = 4
} QuestStage;

enum {
    METRIC_HERO_LEVEL     = 1u,
    METRIC_MONSTERS_SLAIN = 2u,
    METRIC_QUEST_STAGE    = 3u,
    METRIC_GOLD           = 4u
};

enum {
    PARAM_DIFFICULTY_MULT  = 1u,
    PARAM_PERSIST_HERO_X   = 100u,
    PARAM_PERSIST_HERO_Y   = 101u,
    PARAM_PERSIST_HERO_HP  = 102u,
    PARAM_PERSIST_LEVEL    = 103u,
    PARAM_PERSIST_EXP      = 104u,
    PARAM_PERSIST_GOLD     = 105u,
    PARAM_PERSIST_SLAIN    = 106u,
    PARAM_PERSIST_QUEST    = 107u
};

enum {
    AI_EVENT_HEAL_HERO   = 1u,
    AI_EVENT_SPAWN_GOLD  = 2u
};

/* ==========================================================================
 * Shared Popup & Dialogue Structures
 * ========================================================================== */

#define MAX_WORLD_POPUPS 32u
#define MAX_DIALOG_PAGES 3u

typedef struct WorldPopup {
    bool active;
    float x;
    float y;
    float vy;
    float timer;
    float duration;
    float font_size;
    FgeColor color;
    char text[64];
} WorldPopup;

typedef enum DialogAction {
    DIALOG_ACTION_NONE = 0,
    DIALOG_ACTION_START_QUEST = 1,
    DIALOG_ACTION_FAIRY_HEAL = 2,
    DIALOG_ACTION_COMPLETE_QUEST = 3
} DialogAction;

typedef struct DialogueState {
    bool active;
    char speaker[48];
    char pages[MAX_DIALOG_PAGES][120];
    uint32_t page_count;
    uint32_t current_page;
    uint32_t visible_chars;
    uint32_t total_chars;
    float type_timer;
    float blip_timer;
    float auto_wait_timer;
    DialogAction on_close_action;
} DialogueState;

/* ==========================================================================
 * Shared RPG Game State
 * ========================================================================== */

typedef struct RpgGameState {
    bool initialized;
    float elapsed;
    float idle_timer;
    bool autopilot_active;

    /* Hero progression */
    uint32_t hero_level;
    uint32_t hero_exp;
    uint32_t hero_exp_next;
    uint32_t gold;
    uint32_t monsters_slain;
    float hero_hp;
    float hero_max_hp;
    float hero_atk;
    FgeVec2 hero_pos;
    FgeVec2 hero_dir;

    /* Quest & Boss status */
    QuestStage quest_stage;
    bool boss_active;
    bool boss_defeated;
    float boss_hp;
    float boss_max_hp;

    /* Audio & Camera */
    uint32_t current_bgm;
    float cam_x;
    float cam_y;
    float cam_zoom;
    float shake_trauma;
    float shake_time;

    /* Dialogue & Floating World Text */
    DialogueState dialogue;
    WorldPopup popups[MAX_WORLD_POPUPS];
} RpgGameState;

extern RpgGameState g_rpg;

/* ==========================================================================
 * Shared Utility & Module APIs
 * ========================================================================== */

/* main.c */
void rpg_add_camera_shake(float amount);
void rpg_sync_persisted_params(void);
void rpg_add_popup(float wx, float wy, float font_size, FgeColor color, const char *text);
void rpg_spawn_burst(float x, float y, uint32_t count, float speed, float ttl, float size, FgeColor col);
float rpg_clamp_f32(float v, float lo, float hi);
void fge_on_reload(void);

/* world_map.c */
void world_map_init(void);
void world_map_update(float dt);
void world_map_shutdown(void);
void world_map_resolve_movement(float *x, float *y, float radius);

/* hero.c */
void hero_init(float start_x, float start_y, float start_hp);
void hero_update(float dt, bool allow_control);
void hero_shutdown(void);
void hero_apply_damage(float dmg, FgeVec2 knock_from);
void hero_heal_full(void);
void hero_gain_exp_gold(uint32_t exp_amt, uint32_t gold_amt);
void hero_trigger_slash(void);
void hero_trigger_dash(void);
void hero_set_move_intent(float dx, float dy);
uint32_t hero_current_slash_id(void);
FgeEntity hero_get_entity(void);

/* npc_quest.c */
void npc_quest_init(void);
void npc_quest_update(float dt);
void npc_quest_shutdown(void);
bool npc_quest_try_interact(void);
bool npc_quest_has_nearby_npc(void);
void npc_quest_advance_dialogue(void);
void npc_quest_open_elder_dialogue(void);
void npc_quest_open_fairy_dialogue(void);

/* monsters.c */
void monsters_init(void);
void monsters_update(float dt);
void monsters_shutdown(void);
bool monsters_find_nearest_target(float x, float y, float max_range, FgeVec2 *out_pos, bool *out_is_boss);
FgeVec2 monsters_get_golem_pos(void);

/* rpg_ui.c */
void rpg_ui_update(float dt);

#endif /* RPG_SHARED_H */
