# FGE — Fast Game Engine

Rust製の**コードファースト・超高速・軽量な2Dゲームエンジン**です。**ゼロポインタのC11ゲームプレイABI**、**シングルパス `wgpu` インスタンス描画 & 2048×2048 動的PNG/CJKグリフアトラス**、**ロックフリーなオーディオ & チップチューンシンセ/BGM**、**Rust側バルク弾幕エミッター**、**ネイティブCホットリロード**、そして組み込みの**AI Bridge**（GPU不要の決定論的ヘッドレステストランナー + ライブJSON-RPC 2.0 TCP介入サーバー）を備えています。

数万〜数十万のアクティブエンティティをネイティブ速度で処理する必要がある弾幕STG、2DアクションRPG、トップダウン型オープンワールド向けに設計されています。

---

## インストール

```bash
cargo install --path crates/fge --root ~/.local --locked --offline
```

これにより `fge` バイナリが `~/.local/bin` にインストールされ、どのディレクトリからでも `fge run ./my_game` を実行できるようになります（`~/.local/bin` が `PATH` に含まれていることを確認してください）。

新規プロジェクトの作成に必要なのは、`game.toml` と C ソースファイルを含むディレクトリだけです（`fge.h` は組み込まれているため、インクルードパスの指定は不要です）。

---

## アーキテクチャ

FGEは厳格なデータ指向設計（DOD / SoA）、世代管理付きハンドル、`Arc<Mutex<T>>` によるロック競合ゼロ、およびネイティブリリース最適化（`.cargo/config.toml` 内の `opt-level = 3`、`lto = "fat"`、`codegen-units = 1`、`-C target-cpu=native`）を採用した7クレート構成のCargoワークスペースです：

```text
crates/
├── fge-core    # Pure CPU simulation kernel: SoA EntityWorld (with native TTL auto-expiry),
│               # Generational EntityId, EmitterWorld (Rust-side bulk bullet/particle offload),
│               # zero-alloc UiCommandQueue, Camera2D, O(N) SpatialHashGrid, BulkSimulation,
│               # CoreInputState, FrameStatsTracker
├── fge-audio   # Lock-free audio engine (sync_channel -> cpal 0.15.3, ZERO Arc<Mutex<T>>):
│               # 8/16/32-bit WAV loader (FgeSound), procedural chiptune SFX synth,
│               # multi-channel RPG BGM sequencer, automatic headless silent fallback
├── fge-abi     # Thread-local EngineContext + 68 zero-pointer #[no_mangle] extern "C" symbols
│               # matching include/fge.h (entities, physics, collisions, input, AI bridge,
│               # audio, TTL/emitters, PNG textures, immediate-mode Japanese/UTF-8 Text & UI)
├── fge-loader  # Native C11 multi-file compiler driver (gcc / clang / tcc priority, FGE_CC & [build] compiler),
│               # embedded fge.h staging, dlopen LoadedGameModule, opt-in fge_on_reload HotReloader
├── fge-bridge  # GPU-free deterministic run_headless_test() + structured HeadlessTestReport,
│               # WorldSnapshot capture, JSON-RPC 2.0 protocol, single-threaded non-blocking TcpListener
├── fge-render  # Instanced 2D wgpu renderer with Y-sort + Camera2D frustum culling,
│               # 2048x2048 DynamicAtlas (PNG loader + ab_glyph Japanese/CJK & 8x8 ASCII glyph cache),
│               # unified world + screen-space UI pass, windowed (winit) & offscreen targets
└── fge         # CLI binary (`fge bench`, `fge run`, `fge test`), game.toml manifest loader, app loop

include/fge.h               # Public C11 API header (strict -std=c11 -Wall -Wextra -Werror -pedantic, 0 raw pointers)
examples/demo_game/         # Multi-file C11 STG starter demo (game.toml + src/{main,player,enemy}.c)
examples/neon_survivors/    # Neon bullet-heaven survival action showcase (6 C11 modules + auto-play AI)
examples/crystal_chronicle/ # Playable Japanese 2D Action RPG (6 C11 modules, dialog UI, BGM, boss emitters)
```

### コア保証
- **C-ABIを跨ぐ生ポインタゼロ**: ゲーム側のCコードは値型構造体（`fge_str("...")` を介した `FgeTextBuf`）および世代管理付きハンドル（`FgeEntity`、`FgeSound`、`FgeEmitter`、`FgeTexture`）のみを通じてやり取りします。失効または無効なハンドルはパニックすることなく安全に `false` またはゼロ値を返します。
- **`Arc<Mutex<T>>` によるロック競合ゼロ**: シミュレーションとライブ `BridgeServer::poll_frame(&mut ctx)` はメインループ上でシングルスレッド実行され、`fge-audio` は有界のロックフリーな `sync_channel` を通じて `cpal` 出力スレッドへデータを供給します。
- **高スループットのインスタンス描画 & バルクオフロード**: `EntityWorld` のSoA配列はGPUインスタンスバッファに直接パックされ、オプションの `Camera2D` AABBフラスタムカリング、`FGE_FLAG_Y_SORT` による順序付け（`fast` または `composite`）、および `EmitterWorld` によるRust側ネイティブ弾丸スポーンとTTL失効処理を利用できます。

---

## Step 5 のハイライト: オーディオ、バルクエミッター & 日本語テキスト/UI

- **ロックフリーオーディオ (`fge-audio`)**:
  - **WAV再生**: `fge_sound_load_wav(fge_str("assets/hit.wav"))` + `fge_sound_play(sound, vol)` により8-bit PCM、16-bit PCM、または32-bit floatのWAVファイルを読み込み、世代管理された `FgeSound` 検証付きで再生します。
  - **プロシージャル・チップチューンSFX & カスタムトーン**: 8種類の内蔵プリセット（`FGE_SFX_SLASH`、`FGE_SFX_HIT`、`FGE_SFX_EXPLOSION`、`FGE_SFX_PICKUP`、`FGE_SFX_BLIP`、`FGE_SFX_CONFIRM`、`FGE_SFX_HEAL`、`FGE_SFX_LEVEL_UP`）に加え、`fge_audio_play_tone`（`Square`、`Triangle`、`Saw`、`Noise`）を提供します。
  - **ループ対応RPG BGMシーケンサー**: `fge_bgm_play_preset` / `fge_bgm_stop` により `FGE_BGM_VILLAGE`、`FGE_BGM_FIELD`、`FGE_BGM_BATTLE`、`FGE_BGM_BOSS` をシームレスに切り替えられます。`fge test` 実行時やオーディオハードウェアのない環境では自動的にサイレントのヘッドレスモードへフォールバックします。
- **ネイティブTTL & Rust側バルクエミッター (`FgeEmitter`)**:
  - `fge_set_ttl(entity, seconds)` により、短命な斬撃、パーティクル、弾丸がRustの `step_kinematics` 内で自動的にデスポーンされます。
  - `fge_emitter_create_radial(...)`、`fge_emitter_set_position`、`fge_emitter_set_spin`、`fge_emitter_set_active` により、螺旋状・放射状の弾幕パターンをRustの `EmitterWorld` に直接オフロードし、弾丸ごとのC-ABI呼び出しオーバーヘッドを回避します。
- **2048×2048 動的アトラス & ゼロポインタ UTF-8/日本語UI**:
  - `fge_texture_load_png(fge_str("assets/hero.png"))` + `fge_set_texture(entity, tex)` により、PNGスプライトをダーティ矩形アップロード付きでGPUアトラスへ動的にシェルフパッキングします。
  - `ab_glyph` がUTF-8 / 日本語CJKフォントをラスタライズします（システムのCJKフォントを自動探索するか、`game.toml` の `[ui].font` を読み込みます。内蔵の8×8 ASCIIビットマップフォールバック付き）。
  - イミディエイトモードUI呼び出し（`fge_ui_panel`、`fge_ui_bar`、`fge_ui_text`、`fge_ui_number`、`fge_ui_dialog_box`、`fge_world_text`）は、文字列を `FgeTextBuf`（`fge_str("勇者よ！北の遺跡へ！")`） として値渡しし、再利用可能なゼロアロケーションの `UiCommandQueue` へ格納します。

---

## サンプルゲーム

| プロジェクト | ジャンル | 特徴 | 実行例 |
| --- | --- | --- | --- |
| [`examples/demo_game`](examples/demo_game) | アーケードSTG | マルチファイルC11スターター（`main`、`player`、`enemy`）、空間ハッシュ衝突判定、AI Bridgeメトリクス | `fge run ./examples/demo_game` |
| [`examples/neon_survivors`](examples/neon_survivors) | 弾幕サバイバルアクション | ウェーブディレクター、自動照準武器、XPジェム、画面シェイク、ホットリロード、オートパイロットAI | `fge run ./examples/neon_survivors` |
| [`examples/crystal_chronicle`](examples/crystal_chronicle) | 日本語2DアクションRPG | Yソート対応の2200×2000ワールド（村 / 囁きの森 / 北の遺跡）、主人公の剣撃＆ダッシュ、タイプライター風 `fge_ui_dialog_box` を備えた村長＆森の妖精NPC、`FgeEmitter` による螺旋弾を放つスライム/ゴブリン/黒曜石のゴーレムボス、動的BGM、`fge_on_reload`、自動冒険AI | `fge run ./examples/crystal_chronicle` |

---

## CLIの使い方

Cargoでビルド＆実行（またはコンパイル済みの `fge` バイナリを直接呼び出し）できます：

```bash
# 1. Single-draw-call rendering & SoA physics benchmark (100,000 sprites)
cargo run --release -p fge -- bench --count 100000 --frames 300 --offscreen --json
fge bench --count 100000 --cull --vsync

# 2. Run a C11 game project (auto-compiles game.toml sources & hot-reloads on edit)
fge run ./examples/crystal_chronicle
fge run ./examples/neon_survivors
fge run ./examples/demo_game --offscreen --frames 60 --no-hot-reload

# 3. GPU-free deterministic headless test runner (structured JSON report for AI loops)
fge test ./examples/crystal_chronicle --frames 900 --seed 42 --dump-json
fge test ./examples/neon_survivors --frames 600 --seed 42 --dump-json
fge test ./examples/demo_game --frames 600 --seed 42 --dt 0.0166667 --dump-json

# 4. Launch a game with the live JSON-RPC 2.0 AI Bridge enabled (with optional API key auth)
fge run ./examples/crystal_chronicle --bridge 127.0.0.1:7777
fge run ./examples/demo_game --bridge 127.0.0.1:7777 --bridge-api-key secret-token
```

> **コンパイラの選択**: `fge-loader` は `gcc`、`clang`、`tcc` の優先順でコンパイラを選択します（または `game.toml` の `[build] compiler` や環境変数 `FGE_CC` に従います）。また、`include/fge.h` は自動的に組み込まれるため、ゲームプロジェクト側で手動のインクルードパス指定は不要です。

---

## クイックスタート (`game.toml` + `fge.h`)

ゲームプロジェクトは、`game.toml` と1つ以上のC11ソースファイルを含むシンプルなディレクトリで構成されます：

```toml
# my_rpg/game.toml
[game]
title = "My FGE RPG"
width = 1280
height = 720
target_fps = 60
vsync = false

[build]
entry = "src/main.c"
sources = ["src/world.c", "src/enemies.c"]
compiler = "gcc"          # Optional: "gcc" | "clang" | "tcc" (default probe: gcc -> clang -> tcc)

[spatial]
cell_size = 64.0          # SpatialHashGrid cell size in world units (> 0.0)
buckets = 4096            # SpatialHashGrid bucket count (must be a positive power of two)

[render]
y_sort_mode = "composite" # "fast" (z_order + y*1e-4) or "composite" ((layer, y, z) for huge worlds)

[audio]
master_volume = 0.8       # Master gain >= 0.0 (default: 0.8)
muted = false             # Disable audio output if true (default: false)

[ui]
# font = "assets/NotoSansCJK-Regular.ttc" # Optional custom TTF/OTF/TTC font (auto-probes system CJK if omitted)
```

```c
/* my_rpg/src/main.c */
#include "fge.h"

static FgeEntity  g_hero;
static FgeEmitter g_boss_ring;

void fge_init(void) {
    fge_camera_set(640.0f, 360.0f, 1.0f);
    fge_bgm_play_preset(FGE_BGM_VILLAGE, 0.45f);

    g_hero = fge_spawn_sprite(640.0f, 360.0f, 24.0f, 24.0f, fge_rgba(80, 220, 255, 255));
    fge_set_collider(g_hero, 12.0f, 1u);
    fge_set_hp(g_hero, 100.0f);

    /* Rust-side radial bullet emitter (zero per-bullet FFI overhead) */
    g_boss_ring = fge_emitter_create_radial(
        640.0f, 180.0f, 2.0f, 8u, 180.0f, 2.5f, 6.0f, 2u, fge_rgba(220, 80, 255, 255)
    );
    fge_emitter_set_spin(g_boss_ring, 0.0f, 1.2f);
}

void fge_update(float dt) {
    (void)dt;
    float speed = fge_param_get(1u, 260.0f); /* Live-tunable via AI Bridge */
    float vx = (fge_key_down(FGE_KEY_RIGHT) ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_LEFT)  ? 1.0f : 0.0f);
    float vy = (fge_key_down(FGE_KEY_DOWN)  ? 1.0f : 0.0f)
             - (fge_key_down(FGE_KEY_UP)    ? 1.0f : 0.0f);
    fge_set_velocity(g_hero, vx * speed, vy * speed);

    if (fge_key_pressed(FGE_KEY_SPACE)) {
        fge_audio_play_sfx(FGE_SFX_SLASH, 0.8f, 1.0f);
    }

    /* Zero-pointer immediate-mode Japanese RPG UI */
    fge_ui_panel(16.0f, 16.0f, 320.0f, 84.0f, fge_rgba(14, 18, 34, 215), fge_rgba(220, 195, 110, 255), 2.0f);
    fge_ui_text(30.0f, 26.0f, 18.0f, fge_rgba(255, 232, 128, 255), fge_str("勇者 LV.1"));
    fge_ui_bar(30.0f, 58.0f, 220.0f, 14.0f, fge_get_hp(g_hero) / 100.0f,
               fge_rgba(68, 224, 118, 255), fge_rgba(28, 20, 32, 235));

    fge_log_metric(1u, (float)fge_entity_count());
}

void fge_shutdown(void) {
    fge_bgm_stop();
    fge_emitter_destroy(g_boss_ring);
    if (fge_entity_is_alive(g_hero)) {
        fge_entity_despawn(g_hero);
    }
}
```

**ホットリロードのオプトイン (`fge_on_reload`)**: `HotReloader` は `void fge_on_reload(void)` をエクスポートしているCモジュールのみをホットリロードします。`fge_on_reload` を省略したモジュールはホットリロードをスキップするため（`poll_and_reload` が `Ok(false)` を返す）、実行中のシミュレーション状態がクリーンに保持されます。

---

## ライブAI Bridge（TCP上のJSON-RPC 2.0）

`fge run <dir> --bridge 127.0.0.1:7777` で起動すると、FGEはTCP上で改行区切りJSON-RPC 2.0（`NDJSON-RPC`）リクエストを受け付けます：

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"fge.ping","params":{}}\n' | nc 127.0.0.1 7777
```

### Bridge認証 (`--bridge-api-key` / `FGE_BRIDGE_API_KEY`)

`--bridge-api-key <SECRET>` を渡す（または `FGE_BRIDGE_API_KEY=<SECRET>` を設定する）ことで、TCPブリッジでの認証を必須にできます：
- クライアントは `fge.auth`（`{"api_key": "<SECRET>"}`）でTCPセッションを一度認証すると、同じ接続上の後続リクエストではキーを再送せずに実行できます。
- あるいは、個別のリクエストの `params` 内に `"api_key": "<SECRET>"` を渡すこともできます。
- 未認証のリクエストはJSON-RPCエラー `-32003`（`Unauthorized`）で失敗します。

### サポートされているRPCメソッド

| カテゴリ | メソッド | パラメータ | 説明 |
| --- | --- | --- | --- |
| 認証 | `fge.auth` | `{ api_key }` | 後続のRPC呼び出し向けに現在のTCP接続を認証します。 |
| ヘルスチェック | `fge.ping` | `{}` | `ok`、現在の `frame`、および生存中の `entities` 数を返します。 |
| 観測 | `fge.world.snapshot` | `{ limit?: usize }` | `WorldSnapshot`（フレーム、カメラ、エンティティ、衝突、パラメータ、メトリクス）を取得します。 |
| 観測 | `fge.world.query_nearby` | `{ x, y, radius, layer_mask? }` | 円 `(x, y, radius)` 内の `EntitySnapshot` 一覧を返します。 |
| 観測 | `fge.entity.get` | `{ index, generation }` | 生存中の単一 `EntitySnapshot` を取得します（死亡または失効済みの場合は `-32001`）。 |
| 観測 | `fge.param.get` | `{ id, default? }` | 実行時チューニング用の数値パラメータを読み取ります。 |
| 介入 | `fge.entity.spawn` | `{ x, y, w?, h?, color?, vel?, hp?, tag?, collider_radius?, layer_mask?, dialogue? }` | スプライトエンティティをスポーンし、世代管理付き `EntityId` を返します。 |
| 介入 | `fge.entity.despawn` | `{ index, generation }` | エンティティをデスポーンし、会話メタデータをクリアします。 |
| 介入 | `fge.entity.patch` | `{ index, generation, pos?, vel?, size?, hp?, tag?, color?, flags?, dialogue? }` | 生存中エンティティのフィールドを変更、またはNPCの会話テキストを付与・更新します。 |
| 介入 | `fge.input.inject` | `{ key, pressed }` | `CoreInputState` に `FgeKey` の押下/解放イベントを注入します。 |
| 介入 | `fge.event.emit` | `{ kind, target?, arg0?, arg1? }` | Cコードが `fge_poll_ai_events()` で読み取れる `FgeAiEvent` をキューに追加します。 |
| 介入 | `fge.param.set` | `{ id, value }` | Cコードから `fge_param_get()` で読み取れる数値パラメータを設定します。 |
| 介入 | `fge.camera.set` | `{ center_x, center_y, zoom }` | アクティブな `Camera2D` の中心座標とズームを更新します。 |

**エラーコード**: `-32700`（パースエラー）、`-32601`（メソッドが見つからない）、`-32602`（無効なパラメータ）、`-32001`（死亡または失効済みエンティティハンドル）、`-32003`（未認証 / APIキーの欠落または不一致）。

### ヘッドレステストレポート (`fge test --dump-json`)
`run_headless_test` は以下を含む `HeadlessTestReport` JSONオブジェクトを出力します：
- `status`: `"ok"`、`"compile_error"`、`"load_error"`、`"config_error"`、または `"runtime_error"`
- `compile_error`: C11コンパイル失敗時の `{ compiler, stderr, exit_code }`（AIエージェントが構文エラーや型エラーを即座に自己修復可能）
- `sim_timing`: `{ total_ms, avg_step_us, max_step_us }`
- `collision_events_total` & `collision_samples`（最大64件のサンプル `CollisionPair`）
- `world_snapshot`: 最終的な決定論的 `WorldSnapshot`（`fge_shutdown` の直前にキャプチャ）

---

## 開発と検証

```bash
# Check & test all 7 workspace crates individually
for crate in fge-core fge-audio fge-abi fge-loader fge-bridge fge-render fge; do
  cargo check -p "$crate"
  cargo test -p "$crate" --lib
done

# Strict C11 validation of public header and all 3 example games
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic include/fge.h
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include examples/demo_game/src/*.c
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include -I examples/neon_survivors/src examples/neon_survivors/src/*.c
gcc -fsyntax-only -std=c11 -Wall -Wextra -Werror -pedantic -I include -I examples/crystal_chronicle/src examples/crystal_chronicle/src/*.c
```
