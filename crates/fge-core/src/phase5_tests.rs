use std::f32::consts::TAU;

use crate::{
    emitter::{EmitterId, EmitterSpec, EmitterWorld},
    handle::EntityId,
    ui::{FgeTextBuf, UiCommandQueue, UiDrawCmd, truncate_utf8_chars},
    world::{ENTITY_FLAG_COLLIDABLE, EntityWorld},
};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

// ============================================================================
// EntityWorld TTL Countdown & Simultaneous Auto-Despawn Tests
// ============================================================================

#[test]
fn entity_world_ttl_countdown_and_infinite_default() {
    let mut world = EntityWorld::new();
    let e_inf = world.spawn_sprite([10.0, 20.0], [16.0, 16.0], [255, 255, 255, 255]);
    let e_ttl = world.spawn_sprite([30.0, 40.0], [8.0, 8.0], [255, 0, 0, 255]);

    assert_eq!(world.get_ttl(e_inf), Some(0.0));
    assert_eq!(world.get_ttl(e_ttl), Some(0.0));
    assert!(!world.set_ttl(EntityId::INVALID, 1.0));
    assert!(!world.set_ttl(
        EntityId {
            index: 0,
            generation: 0
        },
        1.0
    ));
    assert_eq!(world.get_ttl(EntityId::INVALID), None);

    assert!(world.set_ttl(e_ttl, 1.0));
    assert!(approx_eq(world.get_ttl(e_ttl).unwrap(), 1.0, 1e-6));

    world.step_kinematics(0.4).unwrap();
    assert!(world.is_alive(e_inf));
    assert!(world.is_alive(e_ttl));
    assert_eq!(world.get_ttl(e_inf), Some(0.0));
    assert!(approx_eq(world.get_ttl(e_ttl).unwrap(), 0.6, 1e-5));

    // Expire e_ttl while e_inf stays alive forever.
    world.step_kinematics(0.65).unwrap();
    assert!(world.is_alive(e_inf));
    assert!(!world.is_alive(e_ttl));
    assert_eq!(world.get_ttl(e_ttl), None);
    assert!(!world.set_ttl(e_ttl, 2.0));
    assert_eq!(world.len(), 1);
    assert!(world.debug_assert_invariants());

    // Setting TTL back to 0.0 makes an entity infinite again.
    let e_reset = world.spawn();
    assert!(world.set_ttl(e_reset, 0.2));
    assert!(world.set_ttl(e_reset, 0.0));
    world.step_kinematics(5.0).unwrap();
    assert!(world.is_alive(e_reset));
    assert!(world.debug_assert_invariants());
}

#[test]
fn entity_world_multiple_simultaneous_ttl_expirations_preserve_swap_remove_invariants() {
    let mut world = EntityWorld::new();
    let mut short_ids = Vec::new();
    let mut medium_ids = Vec::new();
    let mut infinite_ids = Vec::new();

    // Interleave short (0.25s), medium (0.60s), and infinite (0.0s) entities across
    // first, middle, and trailing slots to stress reverse-index swap_remove!
    for i in 0..24u32 {
        let id = world.spawn_sprite(
            [i as f32 * 10.0, i as f32 * 20.0],
            [16.0, 16.0],
            [i as u8, 128, 255, 255],
        );
        assert!(world.set_tag(id, 1000 + i));

        match i % 3 {
            0 => {
                assert!(world.set_ttl(id, 0.25));
                short_ids.push((id, i));
            }
            1 => {
                assert!(world.set_ttl(id, 0.60));
                medium_ids.push((id, i));
            }
            _ => {
                infinite_ids.push((id, i));
            }
        }
    }

    assert_eq!(world.len(), 24);
    assert!(world.debug_assert_invariants());

    // First wave: all 8 short-TTL entities expire simultaneously.
    world.step_kinematics(0.30).unwrap();
    assert_eq!(world.len(), 16);
    assert!(world.debug_assert_invariants());

    for (id, _) in &short_ids {
        assert!(!world.is_alive(*id));
    }
    for (id, orig_idx) in medium_ids.iter().chain(infinite_ids.iter()) {
        assert!(world.is_alive(*id));
        assert_eq!(world.get_tag(*id), Some(1000 + *orig_idx));
        assert_eq!(
            world.get_position(*id),
            Some([*orig_idx as f32 * 10.0, *orig_idx as f32 * 20.0])
        );
    }

    // Second wave: all 8 medium-TTL entities expire simultaneously.
    world.step_kinematics(0.35).unwrap();
    assert_eq!(world.len(), 8);
    assert!(world.debug_assert_invariants());

    for (id, _) in &medium_ids {
        assert!(!world.is_alive(*id));
    }
    for (id, orig_idx) in &infinite_ids {
        assert!(world.is_alive(*id));
        assert_eq!(world.get_tag(*id), Some(1000 + *orig_idx));
        assert_eq!(world.get_ttl(*id), Some(0.0));
    }
}

// ============================================================================
// EmitterWorld Bulk Bullet Offload & Generational Handle Tests
// ============================================================================

#[test]
fn emitter_world_spawns_bulk_radial_spinning_bullets_with_ttl_and_colliders() {
    let mut emitters = EmitterWorld::new();
    let mut world = EntityWorld::new();

    let spec = EmitterSpec {
        pos: [640.0, 360.0],
        rate_per_sec: 60.0,
        count_per_burst: 20,
        speed: 240.0,
        bullet_ttl: 0.75,
        collider_radius: 6.0,
        layer_mask: 0b0010,
        size: [10.0, 10.0],
        color_rgba: [255, 80, 120, 255],
        base_angle_rad: 0.0,
        angular_vel_rad: 1.5,
        active: true,
    };

    let emitter = emitters.create_radial(spec);
    assert_eq!(emitters.len(), 1);
    assert!(emitters.is_alive(emitter));

    // Step 1.0s at 60 Hz -> ~60 bursts * 20 bullets = ~1200 bullets spawned!
    let dt = 1.0 / 60.0;
    for _ in 0..60 {
        emitters.step_emitters(dt, &mut world);
        world.step_kinematics(dt).unwrap();
    }

    // With bullet_ttl = 0.75s, roughly 0.75 * 1200 = ~900 live bullets remain,
    // while ~300 older bullets have already auto-despawned via TTL.
    let live_bullets = world.len();
    assert!(
        live_bullets >= 700 && live_bullets <= 1100,
        "expected ~900 live bullets with 0.75s TTL, got {live_bullets}"
    );
    assert!(world.debug_assert_invariants());

    // Verify bullet attributes (colliders, layer mask, speed, TTL, color).
    for idx in 0..world.len() {
        let id = world.entities[idx];
        let flags = world.get_flags(id).unwrap();
        assert_ne!(flags & ENTITY_FLAG_COLLIDABLE, 0);
        assert_eq!(world.layer_mask[idx], 0b0010);
        assert!((world.collider_radius[idx] - 6.0).abs() < 1e-6);
        assert_eq!(world.color_rgba[idx], [255, 80, 120, 255]);

        let vel = world.get_velocity(id).unwrap();
        let speed = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
        assert!(
            approx_eq(speed, 240.0, 1e-2),
            "bullet speed mismatch: {speed}"
        );

        let ttl = world.get_ttl(id).unwrap();
        assert!(ttl > 0.0 && ttl <= 0.75);
    }

    // Spinning emitter should have rotated its base_angle over 1.0s.
    let dense = emitters.emitters.iter().position(|&e| e == emitter).unwrap();
    assert!(
        approx_eq(emitters.base_angle[dense] % TAU, 1.5 % TAU, 1e-2),
        "expected base_angle ~1.5 rad, got {}",
        emitters.base_angle[dense]
    );
}

#[test]
fn emitter_world_position_spin_active_and_destroy_handle_validation() {
    let mut emitters = EmitterWorld::new();
    let mut world = EntityWorld::new();

    let id = emitters.create_radial(EmitterSpec {
        pos: [0.0, 0.0],
        rate_per_sec: 10.0,
        count_per_burst: 8,
        speed: 100.0,
        bullet_ttl: 5.0,
        collider_radius: 4.0,
        layer_mask: 1,
        size: [8.0, 8.0],
        color_rgba: [255, 255, 0, 255],
        base_angle_rad: 0.0,
        angular_vel_rad: 0.0,
        active: true,
    });

    assert!(emitters.set_position(id, 300.0, 400.0));
    assert!(emitters.set_spin(id, 0.5, 2.0));

    // Pause emitter -> no new bullets spawned while inactive.
    assert!(emitters.set_active(id, false));
    emitters.step_emitters(0.5, &mut world);
    assert_eq!(world.len(), 0);

    // Reactivate -> spawns bullets from updated position (300, 400).
    assert!(emitters.set_active(id, true));
    emitters.step_emitters(0.25, &mut world);
    assert!(world.len() >= 8);
    assert_eq!(world.get_position(world.entities[0]), Some([300.0, 400.0]));

    // Destroy emitter and verify stale handle rejection.
    assert!(emitters.destroy(id));
    assert!(!emitters.is_alive(id));
    assert!(emitters.is_empty());

    assert!(!emitters.destroy(id));
    assert!(!emitters.set_position(id, 1.0, 2.0));
    assert!(!emitters.set_spin(id, 0.0, 0.0));
    assert!(!emitters.set_active(id, true));

    // Invalid / zero handles also rejected.
    let zero = EmitterId {
        index: 0,
        generation: 0,
    };
    assert!(!emitters.is_alive(zero));
    assert!(!emitters.is_alive(EmitterId::INVALID));
    assert!(!emitters.destroy(zero));
    assert!(!emitters.destroy(EmitterId::INVALID));
    assert!(!emitters.set_active(zero, true));
    assert!(!emitters.set_position(EmitterId::INVALID, 0.0, 0.0));
}

// ============================================================================
// FgeTextBuf, UTF-8 Truncation & Zero-Alloc UiCommandQueue Tests
// ============================================================================

#[test]
fn fge_text_buf_handles_japanese_utf8_and_safe_overflow_truncation() {
    let jp = "村長：勇者よ、ゴーレムを倒してくれ！";
    let buf = FgeTextBuf::from_str_lossy(jp);
    assert_eq!(buf.len as usize, jp.len());
    assert_eq!(buf.as_str().as_ref(), jp);

    let empty = FgeTextBuf::from_str_lossy("");
    assert_eq!(empty.len, 0);
    assert_eq!(empty.as_str().as_ref(), "");

    // Overlong 3-byte Japanese characters (> 120 bytes): must truncate cleanly on char boundary.
    let overlong = "勇".repeat(50); // 150 bytes
    let truncated = FgeTextBuf::from_str_lossy(&overlong);
    assert!(truncated.len <= 120);
    let s = truncated.as_str();
    assert!(!s.contains('\u{FFFD}'), "split multi-byte UTF-8 character!");
    assert_eq!(s.chars().count(), 40); // 40 * 3 == 120 bytes
}

#[test]
fn truncate_utf8_chars_slices_multibyte_japanese_without_panicking() {
    let jp = "村長：勇者よ、ゴーレムを倒してくれ！";
    let total_chars = jp.chars().count() as u32;
    assert_eq!(total_chars, 18);

    assert_eq!(truncate_utf8_chars(jp, 0), "");
    assert_eq!(truncate_utf8_chars(jp, 1), "村");
    assert_eq!(truncate_utf8_chars(jp, 2), "村長");
    assert_eq!(truncate_utf8_chars(jp, 3), "村長：");
    assert_eq!(truncate_utf8_chars(jp, 7), "村長：勇者よ、");
    assert_eq!(truncate_utf8_chars(jp, total_chars), jp);
    assert_eq!(truncate_utf8_chars(jp, u32::MAX), jp);

    // Mixed ASCII + Japanese
    let mixed = "LV.5 勇者 HP:120";
    assert_eq!(truncate_utf8_chars(mixed, 5), "LV.5 ");
    assert_eq!(truncate_utf8_chars(mixed, 6), "LV.5 勇");
    assert_eq!(truncate_utf8_chars(mixed, 7), "LV.5 勇者");
}

#[test]
fn ui_command_queue_commands_spans_and_zero_reallocation_arena_reuse() {
    let mut ui = UiCommandQueue::new();
    let speaker = "村長";
    let message = "村長：勇者よ、ゴーレムを倒してくれ！";

    // Warm up with a realistic RPG HUD + 100 labels so capacities stabilize.
    let push_frame = |q: &mut UiCommandQueue| {
        q.push_panel(16.0, 16.0, 320.0, 96.0, [20, 24, 40, 220], [240, 200, 90, 255], 2.0);
        q.push_bar(28.0, 48.0, 200.0, 14.0, 0.75, [60, 220, 110, 255], [40, 40, 40, 255]);
        q.push_number(240.0, 46.0, 18.0, 9999, [255, 255, 255, 255]);
        q.push_world_text(420.0, 280.0, 16.0, [255, 220, 80, 255], "EXP +25");
        q.push_dialog_box(80.0, 520.0, 1120.0, 160.0, speaker, message, 9);
        for i in 0..100 {
            q.push_screen_text(28.0, 80.0, 14.0, [255, 255, 255, 255], if i % 2 == 0 {
                "クエスト：黒曜のゴーレムを討伐せよ"
            } else {
                "森の妖精：[SPACE] 話す"
            });
        }
    };

    push_frame(&mut ui);
    assert_eq!(ui.len(), 105);
    assert!(!ui.is_empty());

    // Validate stored commands & span resolution.
    match &ui.commands()[2] {
        UiDrawCmd::ScreenText { text_span, .. } => {
            assert_eq!(ui.resolve_span(text_span.clone()), "9999");
        }
        other => panic!("expected ScreenText for push_number, got {other:?}"),
    }
    match &ui.commands()[3] {
        UiDrawCmd::WorldText { text_span, .. } => {
            assert_eq!(ui.resolve_span(text_span.clone()), "EXP +25");
        }
        other => panic!("expected WorldText, got {other:?}"),
    }
    match &ui.commands()[4] {
        UiDrawCmd::DialogBox {
            speaker_span,
            message_span,
            visible_chars,
            ..
        } => {
            assert_eq!(ui.resolve_span(speaker_span.clone()), speaker);
            assert_eq!(ui.resolve_span(message_span.clone()), message);
            assert_eq!(*visible_chars, 9);
            assert_eq!(
                truncate_utf8_chars(ui.resolve_span(message_span.clone()), *visible_chars),
                "村長：勇者よ、ゴー"
            );
        }
        other => panic!("expected DialogBox, got {other:?}"),
    }

    // Record backing buffer pointers/capacities after warm-up, then verify zero reallocations across frames!
    let arena_ptr = ui.text_arena.as_ptr();
    let arena_cap = ui.text_arena.capacity();
    let cmds_ptr = ui.cmds.as_ptr();
    let cmds_cap = ui.cmds.capacity();

    for _ in 0..20 {
        ui.clear();
        assert!(ui.is_empty());
        assert_eq!(ui.text_arena().len(), 0);
        push_frame(&mut ui);
        assert_eq!(ui.len(), 105);
        assert_eq!(ui.text_arena.as_ptr(), arena_ptr);
        assert_eq!(ui.text_arena.capacity(), arena_cap);
        assert_eq!(ui.cmds.as_ptr(), cmds_ptr);
        assert_eq!(ui.cmds.capacity(), cmds_cap);
    }
}
