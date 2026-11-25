#include "neon_shared.h"

#include <math.h>

typedef struct ParticleSlot {
    FgeEntity entity;
    float age;
    float lifetime;
    float start_size;
    float end_size;
    float drag;
    uint8_t start_alpha;
    bool active;
} ParticleSlot;

static ParticleSlot g_particles[MAX_PARTICLES];
static uint32_t g_particle_cursor = 0u;

void fx_init(void) {
    g_particle_cursor = 0u;
    for (uint32_t i = 0; i < MAX_PARTICLES; ++i) {
        g_particles[i].entity = fge_entity_null();
        g_particles[i].age = 0.0f;
        g_particles[i].lifetime = 0.0f;
        g_particles[i].start_size = 4.0f;
        g_particles[i].end_size = 0.0f;
        g_particles[i].drag = 2.0f;
        g_particles[i].start_alpha = 255u;
        g_particles[i].active = false;
    }
}

void fx_spawn_particle(
    float x,
    float y,
    float vx,
    float vy,
    float start_size,
    float end_size,
    float lifetime,
    float ang_vel,
    FgeColor color
) {
    if (lifetime <= 0.01f) {
        lifetime = 0.01f;
    }

    uint32_t chosen = MAX_PARTICLES;
    for (uint32_t n = 0; n < MAX_PARTICLES; ++n) {
        uint32_t idx = (g_particle_cursor + n) % MAX_PARTICLES;
        if (!g_particles[idx].active || !fge_entity_is_alive(g_particles[idx].entity)) {
            chosen = idx;
            break;
        }
    }

    if (chosen == MAX_PARTICLES) {
        /* Recycle the oldest particle when pool is saturated */
        float best_ratio = -1.0f;
        chosen = g_particle_cursor % MAX_PARTICLES;
        for (uint32_t i = 0; i < MAX_PARTICLES; ++i) {
            float r = g_particles[i].age / g_particles[i].lifetime;
            if (r > best_ratio) {
                best_ratio = r;
                chosen = i;
            }
        }
        if (fge_entity_is_alive(g_particles[chosen].entity)) {
            fge_entity_despawn(g_particles[chosen].entity);
        }
    }

    g_particle_cursor = (chosen + 1u) % MAX_PARTICLES;

    FgeEntity e = fge_spawn_sprite(x, y, start_size, start_size, color);
    fge_set_velocity(e, vx, vy);
    fge_set_rotation(e, fge_rand_range(0.0f, NEON_TAU));
    fge_set_angular_velocity(e, ang_vel);
    fge_set_z_order(e, 12.0f);
    fge_set_tag(e, TAG_PARTICLE);

    g_particles[chosen].entity = e;
    g_particles[chosen].age = 0.0f;
    g_particles[chosen].lifetime = lifetime;
    g_particles[chosen].start_size = start_size;
    g_particles[chosen].end_size = end_size;
    g_particles[chosen].drag = 2.8f;
    g_particles[chosen].start_alpha = color.a;
    g_particles[chosen].active = true;
}

void fx_spawn_burst(
    float x,
    float y,
    uint32_t count,
    float speed_min,
    float speed_max,
    float size,
    FgeColor color
) {
    for (uint32_t i = 0; i < count; ++i) {
        float angle = fge_rand_range(0.0f, NEON_TAU);
        float speed = fge_rand_range(speed_min, speed_max);
        float s = size * fge_rand_range(0.65f, 1.35f);
        float life = fge_rand_range(0.22f, 0.55f);
        float spin = fge_rand_range(-10.0f, 10.0f);
        fx_spawn_particle(
            x,
            y,
            cosf(angle) * speed,
            sinf(angle) * speed,
            s,
            0.5f,
            life,
            spin,
            color
        );
    }
}

void fx_spawn_hit_spark(float x, float y, FgeColor color) {
    fx_spawn_burst(x, y, 5u, 80.0f, 240.0f, 5.5f, color);
}

void fx_spawn_ring(
    float x,
    float y,
    uint32_t count,
    float speed,
    float size,
    FgeColor color
) {
    if (count == 0u) {
        return;
    }
    for (uint32_t i = 0; i < count; ++i) {
        float angle = (NEON_TAU * (float)i) / (float)count;
        fx_spawn_particle(
            x + cosf(angle) * 14.0f,
            y + sinf(angle) * 14.0f,
            cosf(angle) * speed,
            sinf(angle) * speed,
            size,
            size * 0.25f,
            0.48f,
            6.0f,
            color
        );
    }
}

void fx_spawn_thruster(float x, float y, float dir_x, float dir_y, FgeColor color) {
    float jitter = fge_rand_range(-0.38f, 0.38f);
    float base_angle = atan2f(dir_y, dir_x) + jitter;
    float speed = fge_rand_range(90.0f, 190.0f);
    fx_spawn_particle(
        x + fge_rand_range(-3.0f, 3.0f),
        y + fge_rand_range(-3.0f, 3.0f),
        cosf(base_angle) * speed,
        sinf(base_angle) * speed,
        fge_rand_range(5.0f, 8.0f),
        1.0f,
        fge_rand_range(0.16f, 0.28f),
        fge_rand_range(-6.0f, 6.0f),
        color
    );
}

void fx_update(float dt) {
    float damp = expf(-2.8f * dt);
    for (uint32_t i = 0; i < MAX_PARTICLES; ++i) {
        ParticleSlot *p = &g_particles[i];
        if (!p->active) {
            continue;
        }
        if (!fge_entity_is_alive(p->entity)) {
            p->active = false;
            p->entity = fge_entity_null();
            continue;
        }

        p->age += dt;
        if (p->age >= p->lifetime) {
            fge_entity_despawn(p->entity);
            p->entity = fge_entity_null();
            p->active = false;
            continue;
        }

        float t = p->age / p->lifetime;
        float inv_t = 1.0f - t;

        /* Shrink size smoothly */
        float sz = p->start_size + (p->end_size - p->start_size) * t;
        if (sz < 0.5f) {
            sz = 0.5f;
        }
        fge_set_size(p->entity, sz, sz);

        /* Fade alpha via fge_get_color / fge_set_color */
        FgeColor col = fge_get_color(p->entity);
        float alpha_f = (float)p->start_alpha * inv_t * inv_t;
        if (alpha_f < 0.0f) {
            alpha_f = 0.0f;
        }
        col.a = (uint8_t)alpha_f;
        fge_set_color(p->entity, col);

        /* Exponential velocity drag */
        FgeVec2 vel = fge_get_velocity(p->entity);
        fge_set_velocity(p->entity, vel.x * damp, vel.y * damp);
    }
}

void fx_shutdown(void) {
    for (uint32_t i = 0; i < MAX_PARTICLES; ++i) {
        if (fge_entity_is_alive(g_particles[i].entity)) {
            fge_entity_despawn(g_particles[i].entity);
        }
        g_particles[i].entity = fge_entity_null();
        g_particles[i].active = false;
    }
}
