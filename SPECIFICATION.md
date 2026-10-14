# liteweight-fps-wasm 仕様書

本書は `liteweight-fps-wasm` の実装済み挙動を記述する。ネイティブ版
`liteweight-fps` を WebAssembly（WebGPU）へ移植したもので、ゲームロジックは
ネイティブ版と共通だが、プラットフォーム層（ウィンドウ、音声、設定保存）と
不要になった機能・設定を Web 向けに置き換えている。

- 対象ターゲット: `wasm32-unknown-unknown`
- ビルドツール: Trunk
- 描画: `wgpu` 24（WebGPU バックエンド）/ `winit` 0.30（Web）
- 対応ブラウザ: WebGPU 対応（Chrome / Edge 113+ 等）

---

## 1. 用語

| 用語 | 意味 |
| --- | --- |
| プレイヤー | 一人称視点の操作主体。位置は目（カメラ）の座標。 |
| BOT | 訓練用の標的。頭部・胴・四肢に判定を持つ。 |
| モード | `Free Training` または `Tracking Aim`。 |
| ADS | Aim Down Sights。右マウスボタンで照準（ズーム）。 |
| 内部解像度 | 3D シーンを描画するオフスクリーンテクスチャの実寸。 |
| 表示サイズ | キャンバス（ブラウザウィンドウ）のピクセルサイズ。 |

---

## 2. アーキテクチャ

```
src/
  lib.rs              クレート入口・モジュール宣言・wasm-bindgen start
  app.rs              Web イベントループ（ApplicationHandler）とゲーム進行
  platform.rs         時刻（web_time）と乱数シードのプラットフォーム抽象
  persistence.rs      GameSettings と localStorage / ファイル保存
  physics/mod.rs      AABB と射線交差
  world/mod.rs        アリーナ・トラッキングレンジのメッシュとコライダー
  engine/
    mod.rs            grid_shade（壁面格子の CPU 再現）
    layout.rs         FrameLayout（内部解像度と表示）
    renderer.rs       SceneRenderer（3D/HUD/テキスト/ブリット）
    text.rs           埋め込みフォントのアトラスとレイアウト
    text_render.rs    テキストの GPU パイプライン
    window.rs         WindowRenderer（サーフェス構成・描画・提示）
    audio.rs          合成効果音と Web Audio 再生
  simulation/
    time.rs           FixedStep（固定タイムステップ）
    game_mode.rs      SessionMode とモード別ルール
    player.rs         移動・衝突・視点
    weapon.rs         武器データ・ロードアウト・発射・ADS・拡散
    bot.rs            BOT 挙動・当たり判定・リスポーン
    combat.rs         ヒットスキャン・ダメージ・戦績
  ui/
    mod.rs            HUD・メニュー・BOT メッシュ・スコープ
    menu.rs           タイトル／ポーズの選択状態
assets/
  fonts/Cantarell-Regular.ttf   埋め込みフォント
  shaders/*.wgsl                シーン/HUD/ブリット/テキスト
index.html / Trunk.toml         ホストページとビルド設定
```

公開型 `Scene`（頂点配列＋u16 インデックス）と `Vertex`（位置＋色）は
`world` からクレートルートへ再エクスポートされる。描画はすべて CPU 生成の
インデックス付きメッシュで行い、テクスチャはフォントアトラスのみ。

---

## 3. プラットフォーム層

### 3.1 ホストとキャンバス
- `index.html` に `<canvas id="game-canvas" tabindex="1">` を置く。
- CSS で `width/height` を `100vw/100vh !important` に固定し、キャンバスは常に
  ビューポート全体を占める（ウィンドウ操作による解像度設定は行わない）。
- 画面全体を覆うオーバーレイを 3 種用意する（いずれも `position: fixed`）。
  | id | 用途 | 表示制御 |
  | --- | --- | --- |
  | `#loading` | 起動直後のローディング（スピナー＋ラベル） | 既定で表示。レンダラ初期化完了で非表示、失敗時はエラー文言に差し替え |
  | `#fallback` | WebGPU 非対応の案内 | 既定 `none`。`navigator.gpu` 不在時に表示し、ゲームを起動しない |
  | `#touch-notice` | デスクトップ専用の案内 | `@media (pointer: coarse) and (hover: none)` で表示 |
- ローディング中・エラー中もキャンバスは背面に存在するが、オーバーレイが覆う。

### 3.2 時刻と乱数シード（platform.rs）
- `Instant` / `SystemTime` / `Duration` / `UNIX_EPOCH` は `web_time` を再エクスポート
  （Web では `Performance.now()` / `Date.now()` を使用）。
- `process_id()`: Web では `Math.random() * u32::MAX` による実行ごとの値。
  ネイティブでは `std::process::id()`。戦闘のリスポーン乱数シードと射撃拡散の
  シードに使う。

### 3.3 音声（audio.rs）
- 合成音（後述 §12）は純粋関数 `synthesize_all()` で生成し、ネイティブ版と
  同一のサンプル列（44100Hz モノラル）を出力する。
- 再生は Web Audio API。`AudioContext`（`Fifo` とは無関係）を作成し
  `GainNode` → `destination` へ接続。効果音は `AudioBuffer` にコピーして
  都度 `AudioBufferSourceNode` で再生する。
- 音量は `set_volume`（0.0〜1.0 にクランプ、非有限は 0）で `GainNode` に反映。
- ブラウザの自動再生制限により `AudioContext` は suspended で開始し得る。
  再生時に suspended なら `resume()` を試みる（射撃などのユーザー操作起点）。
- `Audio::new()` 失敗時は「AUDIO UNAVAILABLE - GAME WILL BE SILENT」通知を出し、
  無音で続行する。

### 3.4 設定の永続化（persistence.rs）
- Web: `localStorage` のキー `liteweight-fps/settings.json` に JSON 保存。
- ネイティブ（テスト用）: OS 標準の設定ディレクトリへアトミック書き込み。
- ロード時 `from_json` で値域検証し、不正なら既定値へフォールバック。
  `#[serde(default)]` のため未知キーは無視され、欠落フィールドは既定値になる。

---

## 4. アプリケーションライフサイクル（app.rs）

### 4.1 起動と非同期初期化
- `#[wasm_bindgen(start)] fn start()` → `console_error_panic_hook::set_once()` → `app::run()`。
- `run()` は WebGPU の有無を確認後、`EventLoop::builder().build()` で
  イベントループを作り `spawn_app(app)` する（Web では `run_app` ではなく `spawn_app`）。
- `resumed()` でキャンバス付きウィンドウを生成し、`wgpu` 初期化（`WindowRenderer::new`）を
  `spawn_local` の非同期タスクで実行する。完了時に結果を共有セルへ入れ、`request_redraw`。
- `RedrawRequested` の先頭で `ensure_renderer()` が完了済みレンダラを取り込み、
  内部解像度を表示アスペクトへ合わせ、テキストを準備し、音声を初期化する。
  完了時に `#loading` を非表示にする。初期化に失敗した場合は `#loading` を
  エラー文言（スピナー非表示）へ切り替える。
- 初期状態は `title_screen = true`（タイトル画面）。レンダラ初期化前は
  `#loading` が画面を覆う。

### 4.2 画面状態
| 状態 | 説明 |
| --- | --- |
| 読み込み中 | レンダラ初期化前。`#loading` オーバーレイを表示。 |
| タイトル | FREE TRAINING / TRACKING AIM / SETTINGS を選択。カーソルは解放。 |
| プレイ | ゲーム進行。マウス捕捉あり（ポインタロックでカーソル非表示）。 |
| 捕捉待ち（ポーズ） | プレイ中だが未捕捉。ゲーム内に「CLICK TO PLAY」「PAUSED」を表示し、世界を凍結。 |
| ポーズ（設定） | ポーズ兼設定メニュー。タイトルから開くと「BACK TO MENU」。 |

- ポーズキー（既定 Esc）または Esc で、プレイ→ポーズ、ポーズ→復帰/戻る。
- ホームキー（ポーズ中）で設定を既定へ戻し、実行時設定を再適用する。
- Web ではポインタロック中に Esc を押すとブラウザがロック解除のために消費し、押下は
  ページに届かない。そのため「ポインタロックの喪失」自体をポーズ要求として扱い、
  プレイ中は **Esc 1 回でポーズ（設定）メニューが開く**（§5.1 のロック喪失検知）。
- ポーズ中（未捕捉）の Esc は通常のキー入力として届くので、そのまま復帰/戻るに使う。

#### 捕捉待ち（Click to Play / Paused）
- 条件: プレイ中（タイトルでも設定メニューでもない）かつ `captured == false`。
- 表示: ゲーム内オーバーレイ（§12.7）。
  - ウィンドウがフォーカス中: **CLICK TO PLAY** / `MOUSE LOOK REQUIRES POINTER LOCK`
  - 非フォーカス（タブ切替・最小化など）: **PAUSED** / `CLICK TO RESUME`
- 挙動: この状態ではシミュレーションを停止（世界を凍結）する。キャンバスの左クリックで
  ポインタロックを再取得すると解除され、プレイが再開する。
- カーソル: 捕捉中は非表示、未捕捉（タイトル・各種ポーズ・捕捉待ち）では表示する。
  これは「クリックで再開」導線のために意図した挙動。

### 4.3 固定タイムステップ（simulation/time.rs）
- `FixedStep::new(60, 5)`: 60Hz 固定、1 フレームの最大更新回数 5。
- 実時間の経過を蓄積し、ステップ数を返す。過負荷（上限超過）時は蓄積を捨てて
  上限回数だけ更新する（無限追いつき防止）。
- ループ本体は `1/60` 秒を前提に移動・武器・戦闘を進める。

### 4.4 レンダーループと FPS 制限
- `RedrawRequested` でシミュレーションと描画を行う。ウィンドウが
  suspended / occluded / 0 サイズ、または描画不能なときは何もしない。
- シミュレーションは「プレイ中 かつ ウィンドウフォーカス中 かつ マウス捕捉中」の
  ときのみ進める。未捕捉の間は捕捉待ちオーバーレイ（§4.2 / §12.7）が表示され、
  世界は凍結する。
- `about_to_wait` で:
  - FPS 制限あり: `ControlFlow::WaitUntil(deadline)` で次フレームまで待機。
  - 制限なし: `ControlFlow::Poll` で都度 `request_redraw`。
- Web では `winit` が `requestAnimationFrame` ではなくスケジューラ
  （`setTimeout` 系）で駆動するため、ループ FPS は FPS 制限で制御する。
  一方、画面への提示は WebGPU canvas が `Fifo`（vsync 同期）固定のため、
  **表示 FPS の上限はディスプレイのリフレッシュレート**になる。

---

## 5. 入力

### 5.1 マウス
- 捕捉はポインタロック（`CursorGrabMode::Locked`）で行う。Web では
  `Confined` は非対応。
- 未捕捉時の左クリックで捕捉を試みる（ユーザー操作起点）。フォーカス復帰時にも再試行。
- カーソル表示: 捕捉成功時に `set_cursor_visible(false)`、解除（`release_capture`）時に
  `set_cursor_grab(None)` と `set_cursor_visible(true)`。Web ではポインタロックが
  カーソルを隠すため、捕捉中は実質非表示となる。
- 成否の検証: `set_cursor_grab(Locked)` は Web では成否を返さないため、`try_capture` は
  要求のみを行い、`poll_capture` が毎フレーム `document.pointerLockElement` を確認して
  `captured` を確定する（1200ms でタイムアウト）。
- ロック喪失: ブラウザがロックを解除した（例: Esc）ことを `poll_capture` が検知したら、
  プレイ中なら自動的にポーズメニューを開く（`open_pause_menu`）。これにより Web でも
  Esc 1 回でポーズできる。
- 捕捉中の相対移動で視点更新:
  `yaw += dx * 0.002 * look_multiplier`、
  `pitch = clamp(pitch - dy * 0.002 * look_multiplier * y_sign, -1.55, 1.55)`。
  ただし `dy` は `look_y_sign()`（反転設定）を乗じる。
- `look_multiplier(ads) = mouse_sensitivity * (1 + (ads_sensitivity - 1) * ads)`。

### 5.2 キーバインド
- `BindingAction::ALL`（11 種）: Forward, Backward, Left, Right, Dash, Jump,
  Reload, Pistol, AssaultRifle, Sniper, Pause。
- 既定: W/S/A/D 移動、LeftShift ダッシュ、Space ジャンプ、R リロード、
  1/2/3 武器、Esc ポーズ。
- 使用可能キー: A〜Z、Space、LeftShift、RightShift、Escape、Digit1〜3。
- 再割り当ては重複を拒否する（使用中キーは「KEY ALREADY IN USE」）。

### 5.3 発射/ADS/リロード
- 左ボタン: 射撃（自動武器は保持で連射、他は押下のみ）。
- 右ボタン: ADS 保持。
- リロード: 再装填が優先。満弾・切替中・リロード中は無視。
- 移動キーは「プレイ中以外は押下を無視、解放は常に反映」。
- シミュレーションは「プレイ中かつウィンドウフォーカスあり」でのみ進む
  （マウス捕捉は必須ではない）。

---

## 6. プレイヤー（simulation/player.rs）

### 6.1 定数と調整値
| 項目 | 値 |
| --- | --- |
| 目線高 `EYE_HEIGHT` | 1.6 |
| 身体高 `PLAYER_HEIGHT` | 1.8 |
| 衝突半径 `PLAYER_RADIUS` | 0.3 |
| 歩行速度 | 4.0 |
| ダッシュ速度 | 6.5 |
| ダッシュ遷移 | 0.18 s |
| ダッシュ武器下げ | 0.055 |
| ジャンプ初速 | 4.2 |
| 重力 | 9.8 |
| 足音間隔（歩行/ダッシュ） | 0.45 / 0.30 s |
| ステップ高さ | 0.22 |
| 接地スナップ距離 | 0.22 |
| 空中制御係数 | 1.0 |
| コンタクトマージン | 0.0 |
| ピッチ上限 | ±1.55 rad |
| 投影 near/far | 0.1 / 100 |

初期位置は `(0, EYE_HEIGHT, 0)`、yaw=0、pitch=0、接地状態。

### 6.2 移動
- 前方は yaw 方向、入力は正規化。水平速度は歩行/ダッシュで決定。
- ダッシュ進捗は 0.18 秒で 0→1（入力保持で上昇、解放で下降）。
- ジャンプは接地時のみ。空中は重力で落下。
- 接地中は依然として `y <= EYE_HEIGHT` を床として扱う。

### 6.3 衝突（AABB 近似）
- プレイヤーは半径 `collision_radius + contact_margin` の円筒（高さ 1.8）で近似。
- 水平移動は x/z 軸ごとに処理し、衝突時はステップ（段差）判定、不可なら元位置へ
  戻してその軸の速度を 0 にする（軸スライディング）。
- 接地スナップで段差・坂を降りても接地を維持。着地時に `landed` を返す。
- `COLLISION_EPSILON = 1e-4` で境界の誤接触を防ぐ。

### 6.4 足音
- `FootstepClock` が移動かつ接地中に間隔を計測。ダッシュ中は短い間隔。着地でリセット。

### 6.5 視点とリコイル
- `view_matrix_with(aspect, fov)` は perspective(角度, aspect, 0.1, 100) × look_at。
- `apply_recoil(kick)` は `pitch += kick`（上限でクランプ）。

---

## 7. 武器（simulation/weapon.rs）

### 7.1 データ表
| 武器 | 装弾 | 発射間隔 | 連射速度 | リロード | 胴ダメージ | ADS-in | ADS-out | リコイル | 方式 | ボルト | 腰撃ち拡散 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| Pistol | 7 | 60/180 s | 180 RPM | 1.8 s | 50 | 0.16 s | 0.16 s | 0.045 | セミ | 0 | 0.015 rad |
| AssaultRifle | 30 | 0.1 s | 600 RPM | 2.0 s | 25 | 0.18 s | 0.18 s | 0.010 | フル | 0 | 0.012 rad |
| Sniper | 5 | 60/50 s | 50 RPM | 2.8 s | 100 | 0.40 s | 0.20 s | 0.075 | ボルト | 1.0 s | 0.010 rad |

ヘッドショットは胴ダメージの 2 倍。

### 7.2 ADS と FOV
- ADS 進行は `ads_in_seconds`/`ads_out_seconds` で 0〜1 に補間。リロード中は ADS 不可。
- 武器ごとの ADS 目標 FOV: Pistol 67、AR 65、Sniper 20。
- 実効 FOV: `fov = base_fov + (ads_fov - base_fov) * ads_progress`（`base_fov` は設定値、既定 70）。

### 7.3 発射と冷却
- `fire()` は「リロード中・ボルト中・冷却中・弾切れ」のいずれかで失敗。
- 成功時: 弾を 1 減らし、視覚リコイル=1.0、冷却=発射間隔、ボルト式はボルト残り時間を設定。
- 入力受付: フル `held || pressed`、セミ/ボルト `pressed` のみ。

### 7.4 リロード/ボルト
- `reload()` は「リロード中・ボルト中・満弾」で失敗。成功時 `reload_seconds` を設定。
- 完了時に弾を装弾数まで補給。切替は装填を中断（残弾維持、ボルトは継続）。

### 7.5 拡散とスナイパー整合
- 腰撃ち（ADS 進行 0）時、`ShotSpreadRng` で `hip_fire_spread_radians` の円錐内に
  射線をばらす（`Tracking Aim` では無効）。
- スナイパーは乱数拡散を使わず、腰撃ち時の照準ずれを ADS で収束させる:
  `remaining = (1 - ads)^2.5`、ずれ角は水平 `4°` に `remaining` を乗じたもの。
- `ShotSpreadRng` は xorshift。シードは時刻由来（`from_entropy`）。

### 7.6 武器切替（Loadout）
- 3 スロット（Pistol/AR/Sniper）を個別に保持。既定は AR。
- 切替時間 `SWITCH_SECONDS = 0.32`、前半 `SWITCH_LOWER_SECONDS = 0.14` で旧武器を
  下げ、後半で新武器を上げる（下げ量 `SWITCH_DROP = 1.05`）。
- 切替中は発射不可。`presentation()` が表示武器とオフセットを返す。

### 7.7 銃口位置と揺れ
- `view_model_layout` で HUD 武器・サイト・銃口のスクリーン座標を共有。
- 揺れ: 周波数 1.8Hz、水平 0.016 / 垂直 0.010、ADS で 0.9 低減、応答 10、
  基準速度 6.5。移動速度に応じて振幅が増える。
- `muzzle_position` は「カメラ面 `VIEW_MODEL_MUZZLE_DEPTH = 0.65`」上の銃口に対応する
  ワールド点を返し、これが射線の起点になる。

### 7.8 無限弾薬
- `FireInput` は押下/保持を保持し、描画間に押下・解放されても一度は発射を試みる
  （冷却中のクリックは予約しない）。
- 無限弾薬時は発射前後で弾を装弾数に戻す。

---

## 8. BOT（simulation/bot.rs）

### 8.1 挙動
| 挙動 | 説明 |
| --- | --- |
| Static | 静止。 |
| Strafe | 指定軸に沿ってサイン波で左右移動。 |
| TrackingStrafe | プレイヤーの視線右方向を軸に、ランダム歩数（45〜120）ごとに反転。 |
| Chase | プレイヤー付近へウェイポイント経由で接近し、障害物を回避。 |

- 生成時/リスポーン時に乱数で `yaw`、移動軸、方向、位相を決める（xorshift）。
- 意思決定は 20Hz（`BOT_AI_INTERVAL = 1/20`）。移動は固定ステップ。
- 移動パラメータ: Strafe 2.2、Chase 1.8、Chase 接近距離 4.5〜6.0、Strafe 角速度 0.9。
  Tracking レーン半幅 2.5、中心上限 x 19.14 / z 11.14。
- Chase はマップ固有のウェイポイント（spawn_slot 2/4）と壁回避（接線方向へ退避）を持つ。

### 8.2 当たり判定
- 1 体につき 8 個の判定: 胴（ロフト面）、両腕・両脚（カプセル）、頭（楕円体, ヘッドショット）、
  両足（楕円体）。BOT の yaw に応じて回転して評価する。
- 楕円体・カプセル・胴面それぞれにレイ交差判定を実装。

### 8.3 体力・死亡・リスポーン
- 初期体力 = `bot_hp`（Tracking は設定値の 3 倍）。0 以下で死亡。
  `apply_damage` は被弾で `flash = 0.15` を設定。
- 死亡時 `respawn_remaining = respawn_seconds`（既定 2.0）。`advance_timers` で減算。
- リスポーン位置: プレイヤーから 8m 以上、コライダー非衝突、他 BOT と重ならないこと。
  さらに遮蔽物の裏（line-of-sight 遮断）を優先し、直前の位置を避ける。
- Tracking のリスポーンはプレイヤー周囲の候補（半径 5.5/8.5/11.0、範囲 x±16.5 / z±10、
  最小間隔 2.5）から選ぶ。

### 8.4 リスポーン地点（Free Training）
- 36 点（±20/±22/±12/±11 の格子状座標）から選定。アリーナスケールを乗じる。

---

## 9. 戦闘（simulation/combat.rs）

- `Combat` は `targets: Vec<Bot>`、`last_shot`、`feedback`、`kills` を保持。
- 射程は既定 100。
- 通常射撃 `shoot(origin, direction, walls, damage)`:
  壁と全 BOT 判定のうち最近傍を選び、BOT 命中時はダメージ適用（ヘッドショット 2 倍）。
- 銃口射撃 `shoot_from_muzzle_with_aim(...)`（実ゲームで使用）:
  1. カメラ原点＋照準方向で「レティクルが選ぶ対象」を決定（壁に遮られていれば対象なし）。
  2. 銃口から照準点へ射線を引き、銃口視差を補正。
  3. 壁と BOT のうち手前のものを採用。BOT 命中かつレティクルが同じ BOT を選んでいれば
     ダメージを与える（`feedback = 0.12`）。
- 命中エフェクトは `last_shot.impact` の位置に小立方体を表示。
- キルは `kills` を加算。

### 9.1 モード固有（game_mode.rs）
| ルール | Free Training | Tracking Aim |
| --- | --- | --- |
| リコイル | 有効 | 無効（視覚リコイルのみ 0） |
| 腰撃ち拡散 | 有効 | 無効 |
| 無限弾薬 | 設定に従う（既定 ON） | 設定に従う（既定 ON） |

無限弾薬は `infinite_ammo_enabled(setting) = setting`（全モード共通、既定 true）。

---

## 10. ワールド（world/mod.rs）

### 10.1 アリーナ
- 50m 四方の床、周囲 4 壁（厚み 0.5、高さ 12）。
- 遮蔽物 6 個（3×3×5、x∈{-12,0,12}、z∈{-8,8}）。
- 階段: x=-22.5、幅 3.2、踏面 0.5、蹴上 0.18、10 段、踊り場付き。
- ランプ: x=22.5、幅 3.2、長さ 8、上昇 1.8、32 分割、踊り場付き。
- BOT 数に応じた水平スケール `scale = 0.68 + (clamp(count,1,20) - 1) * 0.08`（0.68〜2.2）。
  非有限値は 1.0 とみなす。
- コライダー（`arena_colliders_scaled`）は同じ形状を AABB で返す。階段・ランプは
  段/分割ごとの AABB。

### 10.2 トラッキングレンジ
- 40×24 m、壁高 8、厚み 0.5 の閉鎖空間。内部遮蔽物なし。
- `tracking_colliders` は床＋周壁の AABB。

---

## 11. レンダリング（engine/）

### 11.1 内部解像度とレイアウト（layout.rs）
- `FrameLayout` は内部解像度・表示サイズ・充填可否を保持。
- 検証: 各辺 1〜4096。倍率は 0.25〜1.0。
- `resize_to_display_scale`: 表示サイズ × 倍率（最大 4096 で制限）を内部解像度にする。
- `viewport()`: 表示全体を埋める場合は全面、そうでない場合は縦横比を保って中央配置
  （レターボックス/ピラーボックス）。0 サイズは `None`（描画休止）。
- 製品版は表示アスペクトに内部解像度を合わせるため、歪みや切れなしに全画面表示する。

### 11.2 描画パス（renderer.rs / window.rs）
- オフスクリーン `Rgba8Unorm` カラーテクスチャ（内部解像度）＋ `Depth32Float`。
- パス順:
  1. シーン: 3D メッシュ（頂点色）、深度 `Less`、カリングなし、背景色 `(0.08,0.10,0.14)`。
  2. エクストラ: 動的ワールドメッシュ（BOT・命中痕）と HUD メッシュ（α ブレンド、深度なし）。
  3. テキスト: フォントアトラス（`R8Unorm`）で HUD/メニュー文字。
  4. プレゼント: 内部テクスチャをサーフェスへ最近傍/フィルタ付きブリット。
- present mode は `Fifo` 固定。サーフェス形式は非 sRGB を優先。
- 動的メッシュの容量: 頂点 8192、インデックス 32768（超過は assert）。
- テキストメッシュの容量: 頂点 4096、インデックス 6144。

### 11.3 シェーダ
- `scene_wall.wgsl`: 3D シーン（`grid_shade` と同一の壁面格子線を WGSL で再現）。
- `scene.wgsl`: HUD（頂点色アルファブレンド）。
- `blit.wgsl`: 提示。
- `text.wgsl`: フォントアトラス。

---

## 12. UI / HUD（ui/mod.rs）

### 12.1 ゲーム HUD（テキスト）
上端にモード名（`FREE TRAINING` / `TRACKING AIM`）、その下に
`武器名  AMMO x/y（無限時は ∞）  KILLS n`。下端に移動・射撃・リロード・武器・ポーズの
キーガイド（現在のバインドを反映）。

### 12.2 HUD メッシュ
- 武器モデル（武器ごとに形状・色を変える）、スナイパーのスコープ、レティクル、
  マズルフラッシュ、命中マーカー、弾数 7 セグ数字（無限時は非表示）、
  リロード/ボルト進行バー。
- 命中マーカーは `hit_marker_strength` で太さ/長さをスケール。
- スナイパースコープ（`ui`）: ADS 進行に応じて開口が拡大し、外周が暗転。
  最大半径 `SCOPE_MAX_RADIUS = 1.90`。周辺暗転は進行 0.3 から増加。

### 12.3 BOT メッシュ
- トルソ（ロフト面）、四肢（リング付きリム）、頭（楕円体＋オレンジ色の額マーカー）、
  足。被弾フラッシュ時は色が明るくなる。yaw で回転。

### 12.4 タイトルメニュー
- 3 項目: `FREE TRAINING` / `TRACKING AIM` / `SETTINGS`。
- 選択項目をオレンジでハイライト。

### 12.5 ポーズ／設定メニュー
- 18 行、スクロール窓（同時表示 11 行、行間 0.095、上端 0.49）。
- 行一覧（インデックス順）:

| # | ラベル | 種別 | 対応設定 |
| ---: | --- | --- | --- |
| 0 | RESUME / BACK TO MENU | 実行 | 復帰/戻る |
| 1 | MOUSE SENSITIVITY | 調整 | adjust 0 |
| 2 | ADS SENSITIVITY | 調整 | adjust 1 |
| 3 | FIELD OF VIEW | 調整 | adjust 2 |
| 4 | INVERT Y AXIS | トグル | toggle 2 |
| 5 | KEY BINDING | 再割当 | — |
| 6 | INTERNAL RESOLUTION | 調整 | adjust 9 |
| 7 | FULLSCREEN | トグル | toggle 1 |
| 8 | FPS LIMIT | 調整 | adjust 10 |
| 9 | INFINITE AMMO | トグル | toggle 0 |
| 10 | SOUND VOLUME | 調整 | adjust 5 |
| 11 | HIT MARKER STRENGTH | 調整 | adjust 11 |
| 12 | BOT COUNT | 調整 | adjust 3 |
| 13 | BOT BEHAVIOR | 調整 | adjust 4 |
| 14 | BOT HP | 調整 | adjust 6 |
| 15 | BOT SPEED | 調整 | adjust 7 |
| 16 | BOT RESPAWN | 調整 | adjust 8 |
| 17 | CHANGE MODE / BACK TO MENU | 実行 | モード変更/戻る |

- 左/右で調整、Enter で決定（トグル/実行）、Home で既定値へ戻す。
- スライダー表示: MOUSE SENSITIVITY / ADS SENSITIVITY / SOUND VOLUME / HIT MARKER STRENGTH。

### 12.6 通知（Notice）
- メッセージは画面中央付近にオレンジで表示し、8 秒で消える。内容:
  - `AUDIO UNAVAILABLE - GAME WILL BE SILENT`
  - `SETTINGS NOT SAVED - CHECK STORAGE`
  - `KEY BINDING UPDATED` / `KEY BINDING CANCELLED` / `KEY ALREADY IN USE`
  - `PRESS A KEY TO REBIND`

### 12.7 捕捉待ちオーバーレイ
- `capture_prompt_overlay(mesh, aspect)`: 画面全体を減光し、中央にパネル
  （`x=-0.40..0.40`、`y=-0.16..0.16`、色 `[0.030,0.040,0.055,0.96]`）と
  左アクセントバー（オレンジ）を描く。
- `capture_prompt_text(mesh, title, subtitle, viewport)`: パネル中央にタイトル
  （中心 y=+0.055、サイズ 0.05、金色）とサブタイトル（中心 y=-0.055、サイズ 0.022、淡色）を
  縦センター揃えで描く。テキストは §4.2 の状態に応じて切り替える。
- HUD メッシュの末尾に追記して描画するため、武器モデル等の上に重なる。

---

## 13. 設定（GameSettings）

| フィールド | 既定 | 値域 / 選択肢 | Web での意味 |
| --- | --- | --- | --- |
| `mouse_sensitivity` | 1.0 | 0.1〜5.0（±0.1） | 有効 |
| `ads_sensitivity` | 0.65 | 0.1〜1.0（±0.05） | 有効 |
| `invert_y` | false | on/off | 有効 |
| `fov` | 70.0 | 60〜120（±1） | 有効 |
| `fullscreen` | false | on/off | Fullscreen API で有効（起動時復元はユーザー操作が必要） |
| `render_scale_index` | 1 | 0.5/0.67/0.75/1.0 | 有効 |
| `fps_limit_index` | 6 | 30/60/90/120/144/240/off | 有効（上限はディスプレイ依存） |
| `key_bindings` | §5.2 | 重複不可 | 有効 |
| `infinite_ammo` | true | on/off | 有効（全モード） |
| `volume` | 0.7 | 0〜1（±0.05） | 有効 |
| `hit_marker_strength` | 1.0 | 0〜1（±0.1） | 有効 |
| `bot_count` | 5 | 1〜20 | 有効 |
| `bot_behavior` | 3 | 0 Static/1 Strafe/2 Chase/3 Mixed | 有効 |
| `bot_hp` | 100 | 50〜200（±10） | 有効 |
| `bot_speed` | 1.0 | 0.5〜2.0（±0.1） | 有効 |
| `bot_respawn_seconds` | 2.0 | 0.5〜10（±0.25） | 有効 |

### 13.1 インデックス空間
- 調整インデックス（`adjust_setting`）:
  `0` マウス感度 / `1` ADS 感度 / `2` FOV / `3` BOT 数 / `4` BOT 挙動 /
  `5` 音量 / `6` BOT HP / `7` BOT 速度 / `8` BOT リスポーン /
  `9` 内部解像度 / `10` FPS 制限 / `11` ヒットマーカー強度。
- トグルインデックス（`toggle_setting`）:
  `0` 無限弾薬 / `1` フルスクリーン / `2` Y 反転。

### 13.2 削除された設定（Web では不要）
- **DISPLAY RESOLUTION**: キャンバスが常にビューポート全体のため `request_inner_size` が
  効かない。
- **VSYNC**: WebGPU canvas は `Fifo` 固定で ON/OFF が挙動に影響しない。

---

## 14. 音声仕様（audio.rs）

`(周波数Hz, 長さs, ノイズ比)` の 9 効果。44100Hz モノラル、振幅 0.18、
エンベロープ `(1 - i/count)^3 * min(t/0.002, 1)`。ノイズは LCG（seed 17）。

| インデックス | 効果 | 周波数 | 長さ | ノイズ |
| ---: | --- | ---: | ---: | ---: |
| 0 | Fire | 110 | 0.075 | 0.7 |
| 1 | Pistol | 240 | 0.06 | 0.65 |
| 2 | Sniper | 75 | 0.18 | 0.8 |
| 3 | Reload | 350 | 0.13 | 0.4 |
| 4 | Hit | 800 | 0.05 | 0.0 |
| 5 | Head | 1400 | 0.07 | 0.0 |
| 6 | Kill | 520 | 0.16 | 0.0 |
| 7 | Footstep | 92 | 0.085 | 0.72 |
| 8 | Land | 58 | 0.16 | 0.78 |

再生ルール: 発射音は武器種別、命中時は `Kill` / `Head` / `Hit` を優先再生、
移動で足音、着地で `Land`、リロードで `Reload`。

---

## 15. ネイティブ版との差異 / 非目標

Web 移植で除外・変更したもの:

- **除外**: `--benchmark`（M0 比較）、`--smoke`、`--fps-log`、`bench` フィーチャ、
  CLI 引数、`Options` / `Run` / `Measurement` / `FpsLog`、M0 ベンチ用シーン、
  Miniquad / sysinfo 依存。
- **設定削除**: DISPLAY RESOLUTION、VSYNC（§13.2）。ポーズの QUIT GAME、
  タイトルの QUIT も削除（終了はタブを閉じる、または CHANGE MODE でタイトルへ）。
- **置換**: rodio → Web Audio、ファイル保存 → localStorage、`std::time` → `web_time`、
  `std::process::id()` → 乱数。
- **Web 固有の追加（UX）**:
  - `#loading` ローディングオーバーレイ（レンダラ初期化の非同期化に伴う）。
  - `#fallback`（WebGPU 非対応）と `#touch-notice`（タッチ端末）の案内。
  - ゲーム内の捕捉待ちオーバーレイ（CLICK TO PLAY / PAUSED、§4.2 / §12.7）。
  - 未捕捉時はシミュレーションを停止する（世界の凍結）。
- **挙動差**:
  - キャンバスは常にビューポート全体（解像度設定は無意味）。
  - present は `Fifo` 固定、表示 FPS はディスプレイのリフレッシュレート上限。
  - present mode を設定する UI は無い。
  - フルスクリーンは Fullscreen API。起動時の復元はユーザー操作が無いため不可。
  - `AudioContext` は suspended 開始があり得るため、初回操作時に resume する。
  - カーソル捕捉はポインタロック（`Locked`）。`Confined` 非対応。
    捕捉中はカーソル非表示、未捕捉（タイトル・ポーズ・捕捉待ち）では表示する。
  - 非表示タブ等ではブラウザによりスロットリングされる。

非目標（移植対象外）: ネイティブ固有のベンチ計測、パッケージ配布、マルチ OS 検証。

---

## 16. ビルドと実行

```sh
# 開発サーバ（ホットリロード）
trunk serve            # http://127.0.0.1:8080/

# リリースビルド（dist/ へ出力）
trunk build --release
```

- `index.html` の `<link data-trunk rel="rust" data-wasm-opt="z"
  data-wasm-opt-params="--enable-bulk-memory --enable-nontrapping-float-to-int" />`。
  binaryen 123 は生成 wasm の `memory.fill`（bulk memory）と
  `i32.trunc_sat_*`（nontrapping-float-to-int）を既定で認識しないため、この 2 機能を
  明示的に有効化して `-Oz` を適用する（約 750KB → 約 670KB）。
  リリースプロファイルは `opt-level = "s"` / `lto = "thin"` / `strip`。
- WebGPU 対応ブラウザで開くこと。非対応時は `#fallback` を、タッチ端末では
  `#touch-notice` を表示する。初期化の間は `#loading` を表示する。

---

## 17. テスト

- ネイティブ `cargo test` で純粋ロジックを検証（現状 42 件）。
  対象: game_mode、weapon/combat/bot のロジック、ui のジオメトリ（タイトル／設定／
  捕捉待ちオーバーレイを含む）、persistence（ファイル経路）、audio（合成）、
  text（アトラス）、physics など。
- `cargo check --target wasm32-unknown-unknown` で Web 経路の型検査。
- ブラウザ実機（WebGPU）での動作確認は手動で行う。
