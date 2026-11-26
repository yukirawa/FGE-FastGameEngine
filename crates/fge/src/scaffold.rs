use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use crate::error::AppError;

const MAIN_C_TEMPLATE: &str = r#"#include "fge.h"

static FgeEntity g_player = { 0xFFFFFFFFu, 0u };

void fge_init(void) {
    fge_camera_set(640.0f, 360.0f, 1.0f);
    g_player = fge_spawn_sprite(640.0f, 360.0f, 48.0f, 48.0f, fge_rgba(80, 200, 255, 255));
    fge_set_collider(g_player, 24.0f, 1u);
}

void fge_on_reload(void) {
    if (!fge_entity_is_alive(g_player)) {
        uint32_t count = fge_query_circle(640.0f, 360.0f, 100000.0f, 1u);
        if (count > 0u) {
            g_player = fge_get_query_entity(0u);
        }
    }
}

void fge_update(float dt) {
    float dx = 0.0f;
    float dy = 0.0f;
    if (fge_key_down(FGE_KEY_W) || fge_key_down(FGE_KEY_UP)) {
        dy -= 1.0f;
    }
    if (fge_key_down(FGE_KEY_S) || fge_key_down(FGE_KEY_DOWN)) {
        dy += 1.0f;
    }
    if (fge_key_down(FGE_KEY_A) || fge_key_down(FGE_KEY_LEFT)) {
        dx -= 1.0f;
    }
    if (fge_key_down(FGE_KEY_D) || fge_key_down(FGE_KEY_RIGHT)) {
        dx += 1.0f;
    }

    if (fge_entity_is_alive(g_player)) {
        const float speed = 280.0f;
        FgeVec2 pos = fge_get_position(g_player);
        pos.x += dx * speed * dt;
        pos.y += dy * speed * dt;
        fge_set_position(g_player, pos.x, pos.y);
    }

    fge_ui_text(24.0f, 24.0f, 28.0f, fge_rgba(240, 244, 255, 255), fge_str("Hello FGE"));
}

void fge_shutdown(void) {
    if (fge_entity_is_alive(g_player)) {
        fge_entity_despawn(g_player);
    }
    g_player = fge_entity_null();
}
"#;

const GITIGNORE_TEMPLATE: &str = r#"*.so
*.dylib
*.dll
*.o
*.obj
*.swp
.DS_Store
"#;

/// Validates that a project name is non-empty and contains only `[A-Za-z0-9_-]`.
pub fn validate_project_name(name: &str) -> Result<(), AppError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(AppError::Io(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "invalid project name `{name}`: must be non-empty and contain only [A-Za-z0-9_-]"
            ),
        )));
    }
    Ok(())
}

fn render_game_toml(name: &str) -> String {
    format!(
        "[game]\ntitle = \"{name}\"\nwidth = 1280\nheight = 720\ntarget_fps = 60\nvsync = false\n\n[build]\nentry = \"src/main.c\"\nsources = []\n"
    )
}

/// Scaffolds a new FGE project directory at `<parent_dir>/<name>`.
///
/// Refuses if `<parent_dir>/<name>` already exists and is non-empty (or is a non-directory).
pub fn scaffold_project(parent_dir: &Path, name: &str) -> Result<PathBuf, AppError> {
    validate_project_name(name)?;

    let target_dir = parent_dir.join(name);
    match fs::symlink_metadata(&target_dir) {
        Ok(meta) => {
            if !meta.is_dir() {
                return Err(AppError::Io(io::Error::new(
                    ErrorKind::AlreadyExists,
                    format!(
                        "target path `{}` already exists and is not a directory",
                        target_dir.display()
                    ),
                )));
            }
            let mut entries = fs::read_dir(&target_dir)?;
            if entries.next().transpose()?.is_some() {
                return Err(AppError::Io(io::Error::new(
                    ErrorKind::AlreadyExists,
                    format!(
                        "target directory `{}` already exists and is not empty",
                        target_dir.display()
                    ),
                )));
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(AppError::Io(err)),
    }

    fs::create_dir_all(target_dir.join("src"))?;
    fs::write(target_dir.join("game.toml"), render_game_toml(name))?;
    fs::write(target_dir.join("src/main.c"), MAIN_C_TEMPLATE)?;
    fs::write(target_dir.join(".gitignore"), GITIGNORE_TEMPLATE)?;

    Ok(target_dir)
}
