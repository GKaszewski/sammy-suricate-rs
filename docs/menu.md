# Sammy Suricate: front-end specification (boot, intro, main menu, options, dialogs, level flow)

Reference for the Rust/Bevy port. It goes deeper than `gameplay.md` §1 and corrects it (see "Corrections" below). Everything is read from `extracted/*.wdl` (CP1250, CRLF) and from the menu level data `extracted/menu_greenpepper.WMB` and the shipped saves.

**Conventions**
- `file:line` cites are against `extracted/<file>.wdl` (original line numbers).
- **[U]** unverified engine semantics (Acknex A4 behaviour not provable from scripts alone), **[I]** inference, **[Q]** quirk or bug (port should decide whether to copy it), **[DEAD]** unreachable in the shipped build.
- Acknex coordinates: X forward, Y left, Z up, quants; angles in degrees (pan, tilt, roll); time in ticks (16/s). `time` in the scripts is the frame time in ticks (about 1 at 16 fps), so "per tick" rates below are multiplied by `time` in the original.
- Entity data comes from `cargo run -q -p sammy-convert -- level <out> extracted/menu_greenpepper.WMB`: `skills[]` = Skill1..Skill8 (the WMB stores only 8), `flags` bit0 = Flag1 (so Flag3 = 4, Flag4 = 8, Flag2 = 2). Skill9 and higher are runtime scratch (`interfac.wdl:5-21`).
- Active build defines (`sammypolish.wdl:5-15`): `422`, `polish`, `frenchfullversion`, `tgawinlose`, `greenpepper`, `musique1`. `language = 1` (`:7`). `shareware`, `demoversion`, `realnet` etc. are not defined, so all their branches are [DEAD].
- Registration: treat as registered. `Ver1.SAV` stores `Siger = 1` (var id 277) and `NumberLast{Tut,Jump,Think}LevelOrg = 12 / 28 / 44`, `LevelEnCours = 999` (decoded; see §7.1). `Singer = Siger` at boot (`sammypolish.wdl:310`).

## Corrections to `gameplay.md` §1

| # | `gameplay.md` says | Actual |
|---|---|---|
| 1 | Play shown when `NeverPlayed>0` (implied per session) | `NeverPlayed` is a **saved** global (`skills.wdl:72`, in `BestSco0.SAV`). It is `+=1` in `GestionPartie2` (`mm1.wdl:532`). The Play item is therefore hidden only until the very first level start **ever** (`interfac.wdl:61`). |
| 2 | Bonus item shown if `BonusActif==2` or `==1 && ExtroVue` | Correct as code, but in the shipped data **every** `levelN.SAV` has `BonusState = 0`, so `BonusActif = 0` and the Bonus item (and the 4 bonus icon slots) are permanently hidden [DEAD in shipped data] (§3.7). |
| 3 | Locked icon: click gives `beep2` | `LevelNotReady` has a click branch (`interfac.wdl:1107`) but `InitMainMenu` sets `enable_Click = 0` for locked icons (`:140`, `:158`), so the click never fires [Q]. |
| 4 | Y/O keys "do nothing" in Yes/No | Y, O and Esc all end the wait loop with no result: the dialog closes and **nothing** happens (`options.wdl:687`, `:738-759`). Only J and N answer. |
| 5 | Tooltip "Szkolenie N :" + title | The second tooltip line (`Titre*_Str`) is an **empty string** for every level (`panels.wdl:82-190`). Only the first line is ever visible. |
| 6 | `Facilite++` has "no live consumer" | Confirmed, with a nuance: `Facilite` feeds `VitesseBonusMin/Max` (`InitLvl.wdl:127-128`), but those two variables are never read anywhere else. |
| 7 | "levels 21/37 unlocked once level 20/36 is unlocked" | Correct, and it is evaluated on the **unlocked** state of 20/36 (value 2), not on having beaten them (§3.8). |
| 8 | Extro when both 28 and 44 completed | The test is `LevelsInfo[28]==2 && LevelsInfo[44]==2`, i.e. both **unlocked** (reached), and `PreviousLevel` is 28 or 44 (§7.4). |
| 9 | Version text visible on the menu | `Texte_Version` is switched off on the first click and **not** switched back on when the menu loop resumes (`interfac.wdl:416`, `:442`, `:430`) [Q]. |
| 10 | Level titles "all level titles from the WDL" | Table in §3.9. Tutorial titles have a colon ("Szkolenie N :"), others do not. |

---

## 1. Boot to menu

### 1.1 Order of events (`ACTION Main`, `sammypolish.wdl:221-600`)

| Step | What | Cite |
|---|---|---|
| 0 | `On_Esc = null`. Show `Texte_CheckingSystem` (pos (4,24), font `Fnt_Help`): **"Sprawdzanie systemu..."** = "Checking system...". | `:227-229`, `panels.wdl:258-263`, `SamTexts.wdl:215` |
| 1 | Render setup: `Render_Inflate`, `max_Particles=2000`, `Max_entities=1000`, `Floor_range=128`, `D3d_panels=2`. If depth is 16, try 800x600 (mode 7) then 640x480 (mode 6). | `:231-265` |
| 2 | For `V5 = 1..49`: `Load_Info "Level",V5; LevelsInfo[V5] = LevelsExist[V5]`; count `NbrLevelsWin` where value is 2; `BonusActif = BonusState` while it is still 0. | `:270-282` |
| 3 | `LevelEnCours = 0`; randomise `MusicOrder[2..49]` to 1..`NbrMusic`(8). | `:284-290` |
| 4 | `Load_Info "Ver",1` (sentinel `LevelEnCours = 999`), `waitt 1`, copy `NumberLast*Level = ...Org`, wait up to 128 ticks for `LevelEnCours == 999` (re-rolling `MusicOrder` every tick), `waitt 8`. | `:291-308` |
| 5 | `Singer = Siger`, copy `UserName` to `CurrentUserName`, `Call SetUserName` (blank string in this build), then fix the music order (levels 1-12 alternate 1/2, 17-20 and 33-36 random 1..3). | `:310-345` |
| 6 | `LevelEnCours = 0; Call SearchNextLevel` (first level with `LevelsInfo != 0`, i.e. level 1) then `ValideLevel` (marks it 2 and saves). | `:348-354` |
| 7 | `LOAD_LEVEL <menu_greenpepper.WMB>`; `EtapesIntro = 999`; `Fog_color = 4`, `Camera.fog = 100` (black); wait 8 ticks; `MenuOptionsGame = 0`. | `:357-372` |
| 8 | If `LevelEnCours < 0 || > 49` set it to 1. If it is **0** (no level files at all) show `Texte_NOLEVEL` centred, font `Fonte_grostitre` (lucida38 25x38), 32 ticks: **"WARNING / I FOUND / NO LEVEL"** (English, 3 lines). | `:377-392`, `SamTexts.wdl:195`, `panels.wdl:324-331` |
| 9 | `Load_Info "BestSco",0` (restores `LevelEnCours`, best scores, checkpoints, sliders, video mode, `NeverPlayed`, `LastOptionChoose`, ...). `waitt 2`; `Sound_Vol = Slider_SoundVol`; `Midi_Vol = Slider_MusicVol`. | `:394-398` |
| 10 | `Tempo_LoadLevel = 12 if AverageFps < 10 else 4`. Apply the stored video mode (`VideoModeVoulu`, 0 means mode 7); `AdaptPanels`; `InitPosPanelsOptions`. | `:403-419` |
| 11 | Measure fps for 32 ticks. If depth 8 and fps < 8, drop to a lower mode. If depth 16 and `AverageFps < 10`, show `Texte_Warning1` (centred at x = half screen, y = 20, font `FonteF_1216`) until any key or 128 ticks: English **"THE GAME SEEMS TO BE VERY SLOW, IT IS ADVISABLE TO QUIT (F10), AND START IT IN SOFTWARE MODE."** (3 lines). | `:545-588`, `SamTexts.wdl:204-209`, `panels.wdl:546-552` |
| 12 | Hide "Checking system". `RetryLoad == 0` (always; the `ifdef 1..49` developer jumps are not defined) so `branch IntroSammy`. | `:590-597` |

Port note: steps 1, 10-11 (video mode and fps probing) are irrelevant. Keep: load saves (§7.1), pick the starting level, show "Sprawdzanie systemu..." for the load time, go to the intro.

### 1.2 Music
- Intro: `PLay_Song Musique1, musicvolume` (looping); `Musique1 = sammy01.mid` (`Intro2.wdl:50`, `sons.wdl:66`). `CurrentMusique = 1`.
- Menu (after returning from a level or via `MainMenu`): `PLay_Song_Once Musique1, 0` then `CurrentMusique = 0` (`interfac.wdl:382-383`). `_Once` = plays once, no loop, so the menu goes silent after the song ends [U]. The second argument 0 is the volume parameter; the effective volume is `Midi_Vol` [U].
- After the first intro the song that is already playing simply continues (looping) into the menu.
- Level music: `PlayCurrentmusique` plays `Musique<MusicOrder[LevelEnCours]>` (`mm1.wdl:3935-3946`); `Musique8/9` are the same files as 1/2 (`sons.wdl:66-74`).

### 1.3 Skip keys
- Boot has no skip. The intro main loop ends on **Enter, Esc or right mouse button** (`Intro2.wdl:107`).
- Logos 2 and 3 (publisher, Suricate) also end on Esc / right mouse (`:1047`, `:1060`, `:1083`, `:1107`, `:1126`, `:1140`), but each has a mandatory 24-tick hold that is not skippable (`:1069-1077`, `:1151-1159`). Logo 1 (A5/3DGS) has no key skip.
- After the intro, `While (Key_Esc != 0) wait` prevents the same Esc press from opening Options (`:209`).

---

## 2. Intro (`IntroSammy`, `Intro2.wdl:1-229`)

The intro is **not** a video: it runs inside the menu scene. Three logos are overlaid while a scripted Sammy walks a loop of path nodes and the camera follows him; when he reaches the first node tagged 999 the camera glides to the menu pose.

### 2.1 Setup (`:1-52`)
1. `MenuOptionsGame = 0`; `Fog_color = 4`, `Camera.fog = 100`, `Mip_Flat = 0`.
2. `create <slogoa5.tga>, pov_syn.pos, DisplayLogoA5` (both `d3d_mode` branches identical, `:11-23`). The logo is a billboard entity in the 3D scene (see 2.2).
3. `Camera_PlusTilt = 50` (written as `Camera_PLustilt`; WDL is case-insensitive, so this overrides the `InitCamera1` value 250 for the whole intro [Q], `:25`), `Call SetPosCamera`, then the camera is set to **`CameraPosOrigine`** = (1, -369, 115), `Camera_Syn` = same, `waitt 1`.
4. `CameraState = 1`, `LimitSpeedCamera = 10`, `Camera_TargetZ = Pov_Syn.Z + 50`, `Camera.arc = 70`, `CameraMode = 1`, `TicksPoussiere = 8`, `intensitepoussiere = 1`, `Pov_Syn.PlayerKey = 1`, **`EtapesIntro = 1`** (this releases Sammy's wait loop, `:322-326`), `Camera.Tilt = 90`.
5. Start music (1.2), `waitt 4`, then an 8-tick loop (`timer = 8`) running `Call CameraHandle_Syn; Call HandleTitles` each tick.
6. Then: `Sun_Light = 100`, `sun_angle.Pan = 250`, `.Tilt = 40`, `TitlesOn = 1`, `camera.fog = 0`, `fog_color = 0`, `Floor_range = 64`, **`PartieEnCours = 1`** (starts the scenery scorpions, `lion.wdl`), **`PlayerInvincible = -1`**, `Camera3Friction = 0.1`, `Camera3VertFriction = 0.5` (`:95-106`).

### 2.2 The three logos (`DisplayLogoA5`, `:924-1181`)
All three are the **same entity** morphed in turn (`slogoa5.tga` sprite, `publogo.mdl`, `surilogo.mdl`). Properties: `near=1`, `Albedo=0`, `passable`, `ambient=100`, `unlit`, `Facing=1`; with `d3d_mode > 0` (assume hardware) fades use `Alpha` + `Transparent`. Position every tick via `PlaceTitle` (`:834-844`): world pos = camera pos + `VECROTATE((Skill2, 0, 0), camera.pan)`, i.e. **Skill2 quants straight ahead of the camera** at camera height, and `pan/tilt = camera pan/tilt` (always facing the viewer).

| Phase | Entity | Duration (ticks) | Behaviour |
|---|---|---|---|
| Setup | morphs publogo -> surilogo -> slogoa5.tga (1 tick each), `camera.fog = 90`, `Skill2 = 50`, `Skill6 = 100`, scale 1 | 3 | `:925-960` |
| A5 fade-in | A5 logo | 25 (`Skill6` 100 down to 3 at 4/tick) | `Skill2 += 10/tick` capped at 285 (so it slides from 50 to 285 ahead). `Alpha = 100 - Skill6` (0 to 97). `camera.fog = 90 - Alpha*0.25` (`:961-976`) |
| A5 hold | | 24 | `Alpha = 100`, opaque, `Skill2 += 5/tick` (cap 285), `camera.fog -= fog*0.05` per tick (`:979-993`) |
| A5 fade-out | | 30 (`Skill6` 31 down to 1) | `Alpha = Skill6*3.125` (96 down to 3), `camera.fog -= fog*0.25` per tick (`:995-1015`) |
| Gap | | 8 | `camera.fog = 0`, `fog_color = 0`, invisible, `waitt 8`, scale becomes **1.5** (`:1016-1026`) |
| Publisher | `publogo.mdl` | fade-in until `Alpha >= 100` at +10/tick (cap 64 ticks, 10 in practice); hold 32 (skippable); **forced hold 24**; fade-out at -20/tick to `Alpha <= 4` (cap 16 ticks, 5 in practice); then invisible | `:1033-1096` |
| Suricate | `surilogo.mdl` | invisible gap 32 (skippable); fade-in +5/tick (cap 64, 20 in practice); hold 64 (skippable); **forced hold 24**; fade-out -15/tick (cap 32, 7 in practice); `ent_purge`, `remove` | `:1102-1181` |
| Total | | about 90 + 71 + 147 = **308 ticks (19 s)**, concurrent with the walk | [I] |

Skill2 stays at its cap 285 during the publisher and Suricate phases. Software-render (`d3d_mode == 0`) variants skip the fades (`Skill6 = 48` instead of 64, no alpha); not needed.
[Q] `PlaceTitle` reads the global `Title_Syn`, which `Title_Hit` also assigns; a hint title firing during a logo would move the logo. Not reachable in practice (the first title fires about 20 s in).

### 2.3 The scripted Sammy (`InitPLayerPatrol :231-329`, `PlayerPatrol :334-385`, `PathPlayerScanned :400-410`, `MoveSammyonPath :412-456`)

Entity (from the level): `suricat1.MDL` at (2790.5, -1073.7, 193.7), pan 180, scale **8**, flags 28 (Flag3/4/5), ambient 80, skills `[0, 15, 0, 0, 600, 0, 0, 0]`.
- Skill2 = `SpeedBalle` = **15**, Skill5 = `BlobZoneScan` = **600** (`mm1.wdl:315`, `:213`).
- `pov_syn = player_syn = my`, `push = -3`, `ambient = 0`, `Albedo = 0`, `Shadow = 1`, `Skin = 1`, `fat = 1`, `narrow = 0`, blob shadow `ombblob2.pcx` (`InitOmbreSammy`). Z is snapped to ground + `HeightFootPlayer*8` by `Sonar`. Flag4 stores the camera offset in `BlobCamDeltaX/Y/Z` (unused by type 3).
- Initial state: `PlayerEtat = 1`, `PlayerPatrolEtat = 0`, `PlayerAnimEtat = EtatAnimWalk` (walk cycle), `PlayerDestpan = pan`, then one scan, then **`EtapesIntro = 0; wait while EtapesIntro == 0`**, then `Branch MoveSammyonPath` (`:311-328`). `IntroSammy` sets `EtapesIntro = 1` which starts him.
- Boot order caveat: `Main` sets `EtapesIntro = 999` just after `LOAD_LEVEL` (`:361`). Whether Sammy's init already ran is [U]; the port should start the walk when the intro starts.

**Per-tick loop (`MoveSammyonPath`, while Flag1 = 1, which is never cleared, so Sammy keeps walking after the intro):**
1. `Call PlayerPatrol` (below).
2. `Pan += Ang(PlayerDestpan - Pan) * 0.25 * time` (smooth heading).
3. `PlayerIntention = (0, 1)` (full forward, no turn); `Call HandleSpeedPlayer`. Forward speed `VitesseRaq.X = 1 * time * SpeedBalle = 15 * time` quants per tick (local-X `Move`, `player.wdl:1156`, `:1397`) [U: exact `Move` smoothing]. Walk animation advances `0.5 * VitesseRaq.X` per tick (`PlayerAnimSpeedWalk 0.5`, `anims.wdl:361-366`) and emits walk dust particles (`DustUnderSammy`).
4. If `BlobZoneScan != 0`: `SCAN` sphere range 300, id **25** (camera zones and hint titles).
5. `SCAN` from Sammy's pos/angle, cone `pan 60, tilt 90`, range 600, id **30** (path nodes).
6. `AnimPlayer`; `Accel VitesseRaq, 0, 0.8`.

**Path nodes (`InitPathplayer :392-398`):** 45 entities `setpath.wmb`, morphed to `small.pcx`, invisible, passable, `enable_scan`. When hit by a scan with id 30 (`PathPlayerScanned`):
```
you.PlayerTargetX = my.x;  you.PlayerTargetY = my.y;
you.PlayerPatrolEtat = 1;  you.PlayerTargetPan = my.pan;
you.playerKey = my.Skill4;
```
Node data: Skill1 = 30 (identity, unused), Skill2 = 20, Skill3 = 2 (unused), **Skill4 = key**, pan = direction toward the next node.

**`PlayerPatrol` state machine** (`PlayerPatrolEtat`, `Angles.z` = horizontal distance to target):
- Etat 0 (no target): `SCAN` from Sammy, range `BlobZoneScan` (600), cone pan 60 / tilt 90, id 30. A hit sets the target and Etat 1.
- Else (target set): `To_angle` from Sammy to target.
  - `dist < 100`: `V = Ang(TargetPan - DestPan) * 0.7 * time`; Etat = 2 (arriving: align with the node's own pan).
  - `dist >= 100 && Etat == 1`: `V = Ang(angleToTarget - DestPan) * 0.5 * time` (turn toward the node).
  - `dist > 600 && Etat == 2`: Etat = 1, `V = Ang(...) * 0.2 * time` (safety).
  - `DestPan += V`.
  - `dist < 50 || (Etat == 2 && V < 5)` (note `V < 5` is also true for any negative V [Q]): Etat = 0; `SCAN` from the **node** position (Sammy's Z), facing the node's pan, range 600, cone pan **20** / tilt 90, id 30, to pick the next node.
- Because step 5 above rescans every tick with a 60-degree cone, the target is continuously re-picked among nodes in front of him. Which node wins when several are in the cone depends on SCAN event order [U].

**Node chain (simulated [I]):** a Python simulation of exactly this algorithm (speed 15/tick, cone 60 degrees / 600, nearest-in-cone or last-in-list; both variants give the same loop) visits these nodes (entity index in the WMB, then the key):
`185(1) 157(1) 184(1) 160(1) 161(1) 162(1) 181(2) 163(0) 189 190 191 192 193 194 195 164 165 196 197 198 166 182 167 183 168(999) 200(999) 201(999) 202(999) 169 170 171 172 187 173 174 188 175 176 186 177 178 179 180 159 158 184(1) ...` (closed loop, about 1950 ticks per lap). All keys are 0 unless shown. First 999 node is reached at tick **about 690** (about 43 s). Port recommendation: store the node polyline as data and simulate or replay it; do not depend on SCAN.

Key values and what they do (`Intro2.wdl:107-128`): `PlayerKey` starts 1 (set by `IntroSammy`). Nodes with Skill4 = 1 keep `EtapesIntro == 1`. While `EtapesIntro == 1` the intro loop reduces `Camera.fog` by `100*time` per tick (to 0, then `fog_color = 0`). The first node with a different key sets `EtapesIntro = PlayerKey` (0 at node 163 after the key-2 node 181; value 2 and 0 have **no other consumer**). Key **999** ends the main loop (`Pov_Syn.PlayerKey != 999`).

### 2.4 Camera during the walk
`CameraHandle_Syn = SuricateCamerahandle` (type 3) because the menu's `InitCamera1` has Flag2 (`mm1.wdl:2778-2784`). Parameters from the entity `initcamera1` skills `[200, 100, 1000, 700, 300, 1000, 90, 250]` -> `HautCamPerso 200`, `HautCamMin 100`, `HautCamMax 1000`, `PersoDistCam 700`, `DistCamMin 300`, `DistCamMax 1000`, `PersoAngArriere 90`, `Camera_PlusTilt 250` (overridden to 50, see 2.1). `initcamera2` skills `[0.05, 0.05, 0.05, 0, 0.1, 0.5, 0, 0]` -> frictions 0.05/0.05/0.05 and vertical 0/0.1/0.5. `IntroSammy` forces `Camera3Friction = 0.1`, `Camera3VertFriction = 0.5` after the first 8 ticks. The follow algorithm is `gameplay.md` §6.2 (`camera.wdl:384-873`). The camera starts at the menu origin (1,-369,115) and chases Sammy, who starts about 2800 quants away, so the first seconds are a long fly-in under the logos.

**Camera zones (scan id 25, radius 300 sphere around Sammy):** 7 `setcamera` entities (`SetCamera`/`SetCameraHit`, `camera.wdl:958-1234`). Each switches the camera type/offsets as in `gameplay.md` §6.4. Skills `[type, dist, height, backAngle, plusTilt, steps, triggerRange, 0]`:

| Entity | Position | Skills |
|---|---|---|
| 16 | (2699.5, -1065.8, 14.3) | 3, 600, 250, -90, 150, 200, 150 |
| 20 | (1739.5, -1065.8, 30.3) | 3, 600, 250, -135, 150, 200, 150 |
| 17 | (505.7, -1465.8, 14.3) | 3, 500, 150, -181, 150, 200, 100 |
| 19 | (-839.3, -1071.2, 30.3) | 3, 500, 150, -180, 80, 300, 100 |
| 18 | (-1110.4, -1663.2, 30.3) | 3, 700, 300, -90, 100, 120, 100 |
| 21 | (-2294.4, -1727.2, 30.3) | 3, 300, 400, 160, 0, 250, 100 |
| 22 | (-2422.4, -879.2, 30.3) | 3, 700, 200, 270, 80, 300, 100 |

### 2.5 Hint title (the only one)
`inittitle` entity `titre.MDL` at (-2208, -1728, 64), scale 1.05, ambient 100, Flag6 (faces camera). Skills `[3, 215.1, 0.007, 0.01, 16, 128, 32, 100]` decode via `InitTitle` (`:492-544`) to: id 3; offset ahead of camera **215** (Y 0, Z 0); anim number `Frc(215.1)*10 = 1` (`LionStand`), anim speed 1/tick, 1 cycle; fade-in speed 16 (alpha 6.25/tick, 16 ticks); lifetime 128 ticks; fade-out speed 32 (alpha 3.125/tick, 32 ticks); max alpha 100; trigger distance 300 (default, `:519-523`); trigger centre = entity pos with Z reduced by 100 (`:496`).
Trigger: Sammy's scan 25 hits it while `TitlesOn != 0` and Sammy is within 300 (`Title_Hit :546-606`). While Sammy stays within 300 it stays; then 128 ticks; then fade-out. It is placed with `PlaceTitle` every tick (`HandleTitles`). In the menu scene the title appears around tick 350-420 when Sammy passes nodes 191-193. Up to 3 titles at once (`Title1_Syn..Title3_Syn`). `Bell1` only plays in-game (`MenuOptionsGame == 1`). Title text/graphic content: see `titre.MDL` (the game title); no Polish text.

### 2.6 End of intro (`:107-229`)
Loop runs while `Pov_Syn.PlayerKey != 999 && EtapesIntro != 999 && !key_enter && !key_esc && !mouse_right`.
1. Hide titles, `TitlesOn = 0`.
2. `MoveMenu` (`interfac.wdl:625-653`): `Mouse_mode = 0`; target = `CameraPosOrigine`/`CameraAngleOrigine`; `TargetMarge = 2`, `CameraAcc = 0.2`. Each tick (until `|Cam.X - T.X| < 2 && |Cam.Z - T.Z| < 2`): `Cam += (Target - Cam) * 0.2 * time` for X, Y and Z, and `Pan`, `Tilt` likewise (no angle wrapping, `Pan` is not `Ang()`-normalised [Q]). Meanwhile `camera.fog -= 5` per tick down to 0.
3. Snap exactly to origin (`:189-204`), `Camera_Syn.passable = 1`, `EtapesIntro = 999` (ends the logo loops), cursor to (screen_width - 200*Ratio.X, 50*Ratio.Y) (overridden next), wait for Esc release, `On_Esc = InitGameOptions`, `Branch GestionJeu` -> `MainMenuSammy`.
4. Sammy keeps walking the node loop behind the menu (the 999 nodes (-3,330), (-216,535), (-322,941), (-644,1032) are in view of the menu camera). After a level -> `MainMenu` the scene reloads and Sammy stands still at his start, because his init waits for `EtapesIntro != 0` and nothing sets it [I].

---

## 3. Main menu scene (`menu_greenpepper.WMB`)

### 3.1 Scene facts
- Sky `sky3.MDL` (`InitSkyMenu`, scale (2,2,1.2), follows the camera in X/Y; `mm1.wdl:3379-3458`), ground `solmenu5.MDL` at (-48,128,-64) scale 2.05 ambient 50, 52 `herbes1.pcx` grass sprites (`InitArbre`, scale 1.5 or 3), 4 `InitMDL` cacti (`cactus1.MDL`), 4 scenery scorpions (`InitLion`, `scorpion.MDL`, positions in `enemy-ai.md`), 15 point lights, `info` entity (sun azimuth 200, elevation 60), `initgame` (`TypeJeu 25`, `ScoreGoal 300`...), `initpublic` (sets `Protection1 = 999`; ignore).
- **Camera origin** (`InitGame2` skills `[3, 1, -369, 115, 90, 1, 0, 0]`, `mm1.wdl:1815-1846`): `CameraPosOrigine = (1, -369, 115)`, `CameraAngleOrigine.Pan = 90`, `.Tilt = 1`. Camera arc 70 (`interfac.wdl:376`, `Intro2.wdl:42`). Pan 90 looks along +Y, so **screen right = +X**, up = +Z.
- `MainMenu` also sets `Sun_Light = 200`, `sun_angle = 250`, `Clip_range = 20000`, `Mip_Flat = Mip_Shaded = 0` (`:367-379`).
- `MainMenuSammy` start (`interfac.wdl:403-427`): `On_Esc = InitGameOptions`; `Camera.fog = 0`, `Fog_Color = 0`; `PanBlack` hidden; `Mouse_Range = 6000` (pick distance); cursor at (`DemiEcranX*Ratio.Y`, `DemiEcranY - 100*Ratio.Y`) = **(256, 160) at 640x480** (`Ratio.Y = Screen_Hgt/600`) [Q: Y ratio used for X]; `Mouse_Spot = (8, 8)`; show `Texte_Version` (and the blank `Texte_UserName`); `ChoixMenu = 0`; `Call Mouse_Toggle`.

### 3.2 Overlay texts
| Text | Pos | Font | String | When |
|---|---|---|---|---|
| `Texte_Version` | (2, 4) | `Standard_font` (ackfont.pcx 6x9) | **"Sammy Suricate V1.42"** | Shown at the start of `MainMenuSammy`, hidden on the first click, not re-shown (`interfac.wdl:416`, `:442`; `panels.wdl:310-315`, `SamTexts.wdl:193`) |
| `Texte_UserName` | (2, 17) | `Fnt_Help` | `UserName_Str` = 48 spaces in this build (`skills.wdl:818`, `InitLvl.wdl:293-297`) | Invisible in effect |
| `Texte_Email`, `Texte_Join` | | | defined, never shown | [DEAD] |

### 3.3 Entities and codes (Skill1)
Every `InitMainMenu`/`InitMainTitle` entity: `Flag1` = invisible (none set), `Flag3` (value 4) = `Ent_AlphaSet(0, 80)` (80 % alpha), `Flag4` (8) = flare (none set). All have `ambient 100`, `Set facing = 1` (sprites and title models face the camera; for the `.wmb` title models whether `facing` actually billboards them is [U]). `waitt 10` before further init.

`Skill1 == 0`: static title (returns right after `facing`; no hover, no pick). `Skill1 != 0`: `enable_Touch/Click/Release = 1`, `passable = 0`, `Event = ObjetMenuSelect`.

#### Root menu items
Positions are in the root view (camera at (1,-369,115), pan 90): screen-x grows with X, screen-up with Z.

| Skill1 | Meaning | Entities (file, pos, Skill2/3/4 spin, Skill7) |
|---|---|---|
| 1 | **Play** (continue) | `titplay.wmb` (4, 560, 96); `titplay2.wmb` (4, 560, 32) pan-spin 7; `titplay3.wmb` (4, 560, 144) roll-spin 7. All Skill7 = 10.03. Hidden (`invisible`, init ends) while `NeverPlayed == 0` (`interfac.wdl:61-65`). |
| 2 | **Tutorial** | `tittutor.wmb` (0, 480, 432); `tittuto2.wmb` (0, 480, 368) pan-spin -4 |
| 3 | **Jump** | `InitMainTitle`: `titjump.wmb` (-384, 604, 344); `titjum2.wmb` (-376, 604, 280) pan-spin 4 |
| 4 | **Think** | `titpuzzl.wmb` (328, 528, 344); `titpuzz2.wmb` (328, 528, 280) pan-spin 4 |
| 5 | **Options** | `titoptio.wmb` (-396, 452, 40) |
| 6 | **Quit** | `titquit.wmb` (512, 668, -136), Skill7 = 0 (does not hover/bob) |
| 50 | **Bonus** | `titbonus.wmb` (256, 668, 96); `titbonu2.wmb` (256, 668, 32) pan-spin 4. Hidden when `BonusActif == 0`, or `BonusActif == 1 && ExtroVue == 0` (`:40-55`). Always hidden in shipped data (§3.7). |
| 20 | **Back** (sub-menus) | 4x `mflechA1.pcx` (82x37 sprite, scale 1.25, Flag3 alpha 80), one per sub-menu, at (-2539.8, 1224.1, -4512.2) Jump, (6100.2, 4824.1, -9168.2) Think, (1124.2, 1224.1, -6576.2) Tutorial, (-5963.8, 1224.1, -4512.2) Bonus |

**Prefab placement [U, verified visually]:** A4 places a `.wmb` map entity with its bounding-box **bottom** at the entity z (the geometry is stored centred). Only then do the titles line up with their diamond halves: each `*2.wmb` diamond is two pyramids with a 48-quant gap for the plate, and e.g. `titjump` (z 344, ±24) and `titjum2` (z 280, ±88) both end up centred at 368. `titquit` (z -136, -320..72) then stands on its post instead of being buried. The port applies this to every prefab whose geometry goes below z 0 (`level.rs` `spawn_object`); doors and platforms already start at z ≥ 0.

Placement in the root view (x right, z up): Jump left (-384, 344), Tutorial centre (0, 432), Think right (328, 344), Play centre (4, 96), Options left (-396, 40), Quit right (512, -136), Bonus (256, 96). Depth Y ranges 452..668.

Sub-menu heading sprites (Skill1 = 0, Flag3 alpha 80, scale 1.5, 256x32): `titlearn.pcx` (1123, 1346.3, -6136) Tutorial, `titjump.pcx` (-2541, 1346.3, -4072) Jump, `titthink.pcx` (6099, 4946.3, -8696) Think, `titbonus.pcx` (99x31) (-5965, 1346.3, -4072) Bonus.

#### Level icons: 3 grids
Icons are camera-facing sprites `iconNNN.pcx` / `ico100NN.pcx` (100x75 px, scale 1.25) laid out on a 144 x 112 quant grid (4 columns, 3 rows). Skill1 = 100 + level. Skill7 = 0 (no bob).

| Skill1 | Level | File | Position | | Skill1 | Level | File | Position |
|---|---|---|---|---|---|---|---|---|
| 101 | 1 | ico10001 | (904, 1256, -6304) | | 117 | 17 | icon101 | (-2760, 1256, -4240) |
| 102 | 2 | ico10002 | (1048, ., -6304) | | 118 | 18 | icon104 | (-2616, ., -4240) |
| 103 | 3 | ico10003 | (1192, ., -6304) | | 119 | 19 | icon103 | (-2472, ., -4240) |
| 104 | 4 | ico10004 | (1336, ., -6304) | | 120 | 20 | icon112 | (-2328, ., -4240) |
| 105 | 5 | ico10005 | (904, ., -6416) | | 121 | 21 | icon107 | (-2760, ., -4352) |
| 106 | 6 | ico10006 | (1048, ., -6416) | | 122 | 22 | icon174 | (-2616, ., -4352) |
| 107 | 7 | ico10007 | (1192, ., -6416) | | 123 | 23 | icon111 | (-2472, ., -4352) |
| 108 | 8 | ico10008 | (1336, ., -6416) | | 124 | 24 | icon172 | (-2328, ., -4352) |
| 109 | 9 | ico10009 | (904, ., -6528) | | 125 | 25 | icon175 | (-2760, ., -4464) |
| 110 | 10 | ico10012 | (1048, ., -6528) | | 126 | 26 | icon177 | (-2616, ., -4464) |
| 111 | 11 | ico10010 | (1192, ., -6528) | | 127 | 27 | icon108 | (-2472, ., -4464) |
| 112 | 12 | ico10011 | (1336, ., -6528) | | 128 | 28 | icon179 | (-2328, ., -4464) |

| Skill1 | Level | File | Position |
|---|---|---|---|
| 133 | 33 | icon106 | (5880, 4856, -8880) |
| 134 | 34 | icon171 | (6024, ., -8880) |
| 135 | 35 | icon110 | (6168, ., -8880) |
| 136 | 36 | icon180 | (6312, ., -8880) |
| 137 | 37 | icon170 | (5880, ., -8992) |
| 138 | 38 | icon102 | (6024, ., -8992) |
| 139 | 39 | icon173 | (6168, ., -8992) |
| 140 | 40 | icon114 | (6312, ., -8992) |
| 141 | 41 | icon178 | (5880, ., -9104) |
| 142 | 42 | icon176 | (6024, ., -9104) |
| 143 | 43 | icon113 | (6168, ., -9104) |
| 144 | 44 | icon109 | (6312, ., -9104) |
| 113-116 | Bonus 1-4 | `b113..b116.wmb` models | (-6184 / -6040 / -5896 / -5752, 1256, -4225.1), scale 1 |

(The Y of the tutorial and jump grids is 1256; the think grid is at Y = 4856.) Tutorial icons 10-12 use files `ico10012`, `ico10010`, `ico10011` because the level files are swapped (`gameplay.md` §3.1). Level 49 (Extro) has no icon; 29-32 and 45-48 have no slot. Codes 129-132, 145-148 do not exist.

#### Camera markers (invisible, passable `setcam.wmb`; actions `posTutorialLevels` etc., `interfac.wdl:903-923`)
| Marker | Position | Pan | Tilt | Skill2/3 (dead pan limits) |
|---|---|---|---|---|
| `PosTutorialLevels` (code 2) | (1121.01, 742.915, -6392.34) | 90 | 0 | 45 / 135 |
| `PosJumpLevels` (code 3) | (-2542.99, 742.915, -4328.34) | 90 | 0 | 45 / 135 |
| `PosthinkLevels` (code 4) | (6097.01, 4342.92, -8968.34) | 90 | 0 | 45 / 135 |
| `PosBonusLevels` (code 50) | (-5966.99, 742.915, -4328.34) | 90 | 0 | 45 / 135 |

The camera is placed **exactly at the marker position with the marker's pan/tilt** (`interfac.wdl:464-468`, `:486-490`, `:508-512`, `:599-603`); arc stays 70. Each grid is ~513 quants ahead of its marker and centred on the marker's X. The root menu scene and the three grids are far apart in the same BSP (the grids are at Z -4000 to -9000, hidden under the ground).

### 3.4 Per-tick hover/bob/spin (`InitMainMenu :74-198`, `InitMainTitle :977-1014`)
For entities with `Skill1 != 0` and `Skill7 != 0`:
```
org = (x,y,z) at init
H = int(Skill7)            ; 10   (Skill12)
D = frac(Skill7)           ; 0.03 (Skill13)
dir = +1; target = org.z + H     (Skill15)
loop each tick:
    z    += (target - z) * D * time
    pan  += Skill2 * time ; tilt += Skill3 * time ; roll += Skill4 * time      (InitMainMenu)
    if |z - target| < 4:                        ; threshold is 8 in InitMainTitle
        dir = -dir ; target = org.z + H*dir
```
Resulting motion (`InitMainMenu`): exponential approach with 3 %/tick; the first leg flips at z = org + 6 after 30 ticks, afterwards it oscillates org + 6 <-> org - 6 with a half period of about 46 ticks (period about 91 ticks, 5.7 s). `InitMainTitle` (Jump heading, threshold 8): org + 2 <-> org - 2, period about 27 ticks, plus `tilt += cos(pan) * Skill3 * time`, `pan += Skill2 * time` (`titjum2` spins at 4/tick; Skill3 = 0 so no tilt). The leftover pan check that flips `Skill17` (`:186-190`) has no effect [DEAD]. Skills 9-18 are scratch; `Skill18 = random(1)*0.05 + 0.001` unused.
Items with `Skill7 == 0` (Quit, Back, level icons) neither bob nor spin.
Whether spin and `facing = 1` combine visibly on the `.wmb` pieces is [U] (the spinning pieces `titplay2/3`, `tittuto2`, `titpuzz2`, `titjum2`, `titbonu2` are decorative companions of the title text).

### 3.5 Mouse picking and hover (`ObjetMenuSelect`, `interfac.wdl:200-327`)
- Pick distance `Mouse_Range = 6000` [U: A4 uses a ray from the camera through the cursor against entities with `enable_Touch`].
- Events: **Touch** = pointer enters, **Release** = pointer leaves, **Click** = left button pressed [U]. Ignored while `PanYesNo.visible == 1`.
- **Touch** (only if `Flag1 == 0`; sets `Flag1 = 1` as the hover latch):
  - sound `Toc2` (= `Cligno1.wav`) at volume 25 (`sons.wdl:13`);
  - entity light: `LightRed/Green/Blue = 255, 180, 50` (orange), `LightRange = 300`;
  - if joypad mode (`JoyPad == 1`) and Joy_1 is down: select immediately (below);
  - if icon (`Skill1 > 100`): `scale_X *= 1.2`, `scale_Y *= 1.2` (so 1.25 -> 1.5), `Call DisplayTitreLevel`, then loop each tick while `Flag1 == 1`: update tooltip positions (3.9) and emit one sparkle particle (below). Non-icon items do nothing more (no scale change).
- **Release** or **Click**: `Flag1 = 0`, hide both tooltip texts, `LightRange = 0`; icons: `scale = 1.25` (absolute).
- **Click**: `EtapesIntro = 0` (stops intro anims), sound `Ding1` (= `tocbois8.wav`) at 50, **`ChoixMenu = Skill1`**.
- Sparkles (icons only): each tick `Angles = (Skill20, Skill21, 0)`; `model_pos = my.pos + VecRotate((64, 5, 0), Angles)`; `EMIT 1, model_pos, particle_interface1`; then `Skill20 += 40*time`, `Skill21 += 15*time` (Skill20/21 start at 0 and keep accumulating across hovers). `particle_interface1` (`particul.wdl:1173-1206`): speed random in [-2, 2] on each axis; colour R 230+rand(20), G 50+rand(50), B rand(50); size 200 + rand(50) shrinking 10 per tick (dies at <= 1, about 20-25 ticks); `Speed.Z += 0.1*time`; flare; bitmap one of `SmallStar0A / 2A / 4A` at random.
- Locked/absent icons use `LevelNotReady` instead (3.8).
- Clickable set: all entities with Skill1 != 0 and `enable_Click` = 1: Play (3 pieces), Tutorial (2), Jump (2), Think (2), Options, Quit, Bonus (2, hidden), 4 Back arrows, and unlocked level icons.

### 3.6 `ChoixMenu` state machine (`MainMenuSammy`, `interfac.wdl:403-624`)
`debut:` loop: `while ChoixMenu == 0 { if mouse_mode == 0 { Call mouse_toggle }; waitt 1 }`. On exit: hide version/username text, `CameraSpeed = 0`, `Camera1Friction = 0.01`. Every branch except ">= 101" first waits for **left-button release** (`Mouse_Left != 0 || joypad button`).

| ChoixMenu | Action |
|---|---|
| 1 Play | wait release; `EtapesIntro = 0`; `Branch GestionPartie2` (starts `LevelEnCours`, §7.2). |
| 2 / 3 / 4 | Wait release. **Hard cut** camera to the marker (`PosTut/Jump/ThinkLevels_syn`: X, Y, Z, Pan, Tilt, no glide). Cursor to (DemiEcranX, Screen_hgt - 30*Ratio.Y) = (320, 456) at 640x480. Loop `While ChoixMenu == N { Call PanCameraChoose [DEAD, :929]; waitt 1 }` until another click sets a different code, then `goto debut`. |
| 50 | Same as 2/3/4 with `PosBonusLevels`. |
| 5 Options | Wait release; `Call InitGameOptions` (§4); wait while `PanFondOptions.visible == 1`; falls through to `ChoixMenu = 0; goto debut`. |
| 6 Quit | Wait release; `OuiNonQuit` (§5); wait while `ResultChoixOuiNon == 0 && !key_esc`; `ChoixMenu = 0; goto debut` (if Yes, `QuitSammy` has already been branched). |
| 1001 | `Call OuiNonQuit` but nothing sets 1001 [DEAD]. |
| 20 Back | Wait release; camera hard-cut to `CameraPosOrigine` / `CameraAngleOrigine.Pan/Tilt`; `ChoixMenu = 0; goto debut`. (Same cut applies when any other code leaves a sub-menu.) |
| >= 101 | `LevelEnCours = ChoixMenu - 100; Branch GestionPartie2` (no wait for release). |
| 999 | Set by `MainMenu` on entry to interrupt a running level (`:334`), cleared by `MainMenuSammy`. |

While in a sub-menu `Esc` still opens Options (`On_Esc = InitGameOptions`); closing returns to the same view.

### 3.7 Visibility conditions summary
| Item | Condition |
|---|---|
| Play (1) | `NeverPlayed != 0` (saved; first level start ever flips it) |
| Bonus (50) | `BonusActif == 2`, or `BonusActif == 1 && ExtroVue != 0`. `BonusActif` is set in `Main` to the first non-zero `BonusState` found while loading `Level1..49.SAV`. In the shipped files every `BonusState` is 0 (decoded) so the item is always hidden. `ExtroVue = 1` is set when the Extro starts (`Extro.wdl:15`). |
| Bonus icons 113-116 | `LevelsInfo[13..16]`; these save files do not exist, so the value is 0, which makes them invisible (out-of-range levels). |
| Level icons | §3.8 |
| `Singer` | Only used for levels 21/37 and the unlock messages (§3.8). Treat as 1. |

### 3.8 Icon state from `LevelsInfo[N]` (`InitMainMenu :94-175`, `LevelNotReady :1016-1112`)
`LevelsInfo[N]` is a per-boot copy of each file's `LevelsExist[N]`: **0 = no save file (absent), 1 = exists but locked, 2 = unlocked** (reached; set by `ValideLevel`). The `levelN.SAV` files in the install directory (a copy that has been played up to level 5) give: 1-5 = 2, 6-12 = 1; 17 = 2, 18-28 = 1; 33 = 2, 34-44 = 1; 49 = 1; 13-16, 29-32, 45-48 = 0 (no file). A clean install is therefore 1 = 2 and 2-5 = 1 as well [I]; levels 1, 17 and 33 are always 2.

Special cases for `Singer == 1`:
- Level 21: if `LevelsInfo[21] != 0`: `LevelsInfo[21] = 2` if `LevelsInfo[20] == 2` else 1 (the local `V` follows). Level 37 likewise from 36. Evaluated each time the menu inits; since `2` means "unlocked", level 21 opens as soon as 20 is unlocked, i.e. when 19 is won [Q] (`:103-131`).

State -> look and behaviour (`temp = Skill1 - 100`, `V = LevelsInfo[temp]`; `temp > 49` plays `beep1`):
| V | Look | Events |
|---|---|---|
| 2 | normal, scale 1.25, ambient 100 | `ObjetMenuSelect` (hover 1.5x scale, sparkles, tooltip; click starts the level) |
| 1 (and `AllLevelsOn == 0`) | `ambient = 100`, `Transparent = 1` (ghost) | Touch + Release only, `Event = LevelNotReady` (`enable_Click = 0`) |
| 0, level in 1-12 / 17-28 / 33-44 | `ambient = 0`, `Transparent = 1` (darker ghost) | Touch + Release only, `LevelNotReady` |
| 0, other levels (13-16, 29-32, 45-48) | `invisible`, no events | none |

(The `AllLevelsOn` cheat (Shift + V + L, `mm1.wdl:3891-3896`) makes V = 1 icons behave like 2 at the next menu init; not needed.)

`LevelNotReady` on Touch (`:1070-1096`): `Toc2` at 25, light (255, 100, 20) range 300 (darker orange than normal), flag latch, show tooltip (3.9) plus a third text `Texte_LevelNotReady` at (mouse.x - 64, mouse.y + 26), font `FonteF_1216`:
| Condition (Singer = 1, tutorial/jump/think ranges) | String |
|---|---|
| V == 1 | **"Musisz zakończyć poprzedni\npoziom przed rozpoczęciem\nnastępnego poziomu"** ("You must finish the previous level before starting the next one") |
| V == 0 | `LevelNothere_Str` (English): "Go to http://www.suricate-software.com\nto download this level" |
(The `Singer == 0` strings `LevelNothereReg_Str`, `OnlyReg_Str` are [DEAD].) Click would play `beep2` but is never delivered (correction 3).

### 3.9 Tooltip and level title table
Two `TEXT` objects, both font `FonteF_1216` (`polish1017.pcx`, 10x17): `Texte_TitreLevelIcon` (string = `levelN_Str`) and `Texte_TitreIcon` (string = `TitreN_Str` = `""`). Positions, refreshed each tick while hovered:
- unlocked icon (`ObjetMenuSelect`): level text at (mouse.x + 20, mouse.y); title text at (mouse.x - 64, mouse.y + 10);
- locked icon (`LevelNotReady`): level text (mouse.x + 32, mouse.y); title text (mouse.x + 32, mouse.y + 10); message (mouse.x - 64, mouse.y + 26).

`DisplayTitreLevel` (`interfac.wdl:840-902`) selects strings by `Skill1 - 100` (1..49); the strings are in `panels.wdl:81-190`:

| Levels | String | English |
|---|---|---|
| 1 | "Szkolenie 1 :            " (trailing spaces) | Training 1 |
| 2-12 | "Szkolenie N :" | Training N |
| 13-16 | "Bonus 1" .. "Bonus 4" | |
| 17-28 | "Ćwicz zręczność N" (N = 1..12) | Practice dexterity N |
| 29-32 | "Bonus 5" .. "Bonus 8" | |
| 33-44 | "Ćwicz myślenie N" (N = 1..12) | Practice thinking N |
| 45-48 | "Bonus 9" .. "Bonus 12" | |
| 49 | "EXTRO" | |

All `TitreN_Str` are `""`. `SousTitreN_Str` are `" "` except `SousTitre44_Str = "Hint : Yellow is the key"` (used only by the in-level intro card, `panels.wdl:236`, `InitPanelsPret`, which is [DEAD] because `InitPanelsPret` is commented out in `StartintroGame`).

---

## 4. Options overlay (`options.wdl`, `paninter.wdl`)

All coordinates are for the 640x480 design (`Video_mode == 6`). `InitPosPanelsOptions` (`options.wdl:500-538`) offsets every panel by (0,0) at 640x480, (80,60) at 800x600, (192,144) at 1024x768, (320,240) at 1280x960 (and (-48,-32) at 512x384, but the game forces 640x480 while the overlay is open, `:106-110`). In the port, draw the overlay on a centred 640x480 canvas.

### 4.1 Opening, closing, modes
`MenuOptionsGame`: **0** = front-end (intro or menu; `Main :372`, `IntroSammy :3`, `MainMenu :381`), **1** = in a level (`DebutPartie :636`), **2** = Extro (`Extro.wdl:116`).

Open (`InitGameOptions :85-142`), triggered by **Esc** (`On_Esc`: set in `MainMenuSammy`, `StartintroGame`, after the level-start sweep `mm1.wdl:814`, after the intro, and `InitExtroOptions` in the Extro) or the **Options** menu item:
1. `Freeze_mode = 1` (freezes the 3D action [U]); `Camera.fog = 70`, `Fog_color = 4` (dims the scene; invisible at 640x480 because `fondopt1.pcx` is opaque and covers the screen [I]).
2. `ResetPanels` (hides the HUD); `SaveKeys` (stores `On_Esc/On_CUU/On_CUD/On_Enter`).
3. Mouse: in game (1/2) `Call mouse_toggle` (cursor on); in the menu only if `mouse_mode == 0`.
4. `VideoModeChoisi = VideoModeVoulu`; show `PanFondOptions`; `DisplayCurrentOptions`; if `LastOptionChoose == 1` start `CheckSlidersVolume`.
5. Navigation panel: mode 0 -> `PanOptionsNavig2` (Back, Quit); mode 1 -> `PanOptionsNavig1` (Resume, Back, Quit) + `PanRestart`; mode 2 -> `PanOptionsNavig1` (no Restart).
6. `On_Esc = On_Enter = Fin_MenuOptions`; `On_CUU/CUD` are empty actions.

Close (`Fin_MenuOptions :10-71`): `RestoreKeys`, mouse off if in game, `Freeze_mode = 0`, hide all option panels, apply a changed video mode, rebuild the HUD in game, restart the music if the preview song was playing (`AudioPanelOn == 2`), `ResultChoixOuiNon = -1`, hide Yes/No, `AudioPanelOn = 0`, **`Camera.fog = 0; fog_color = 0`** (this also wipes the level's own fog [Q]).

**The first tab shown is Help** (`LastOptionChoose` default 5, global and saved, `skills.wdl:733`); it then remembers the last tab.

### 4.2 Panels and bitmaps (640x480 positions, top-left origin)
| Element | Bitmap (size) | Position | Layer |
|---|---|---|---|
| Background | `fondopt1.pcx` (640x480) | (0, 0). Dark brown; lighter content rectangle about x 330-626, y 14-465 | 1 |
| Tab "Audio" | `audio1.pcx` / `audio2.pcx` (218x80) text **"AUDIO/VIDEO I USTAWIENIA"** | (41, 14) | 2 |
| Tab "Control" | `small.pcx` (4x4, blank) | (41, 105) [DEAD tiny hot spot overlapping the Help tab's corner; leads to the registration page (`LastOptionChoose = 4`)] | 2 |
| Tab "Help" | `help1.pcx` / `help2.pcx` (226x47) **"POMOC"** | (41, 105) | 2 |
| Tab "Credits" | `credits1.pcx` / `credits2.pcx` (226x46) **"TWÓRCY"** | (41, 152) | 2 |
| Resume | `resum1.pcx` / `resum2.pcx` (306x42) **"POWRÓT DO GRY"** | (0, 335) | 2 |
| Back | `back1.pcx` / `back2.pcx` (306x40) **"POWRÓT DO MENU"** | (0, 377) | 2 |
| Quit | `quit1.pcx` / `quit2.pcx` (306x41) **"WYJDŹ Z GRY"** | (0, 417) | 2 |
| Restart (in game only) | `restart2.pcx` / `restart3.pcx` (204x50) **"STARTUJ PONOWNIE"** | (48, 268) | 4 |

Button behaviour [U: A4 `BUTTON x,y,bmapOn,bmapOff,bmapOver,clickAction,releaseAction,touchAction` argument order, consistent with the usage]: image 2 is the normal state, image 3 the hover state, image 1 the pressed state; the 2nd action (`Clic_sound1` = `Ding1`) plays on release. The selected tab shows its "2" (highlighted) image in all three states. Per tab panel (`PANOptions0..6`, `paninter.wdl:392-462`): tab 0 (nothing selected): image1/image1/image2 for all tabs (hover = "2"). In panels 1-6 the selected tab uses image2 in all states, the others image2/image1/image2 (pressed/normal/hover). Resume/Back/Quit: images 1,1,2 (hover = "2").

Tab actions (`options.wdl`): Audio -> `ChooseAudio` (`ding2` = `tocbois6.wav` at 50, tab 1, start slider polling). Help -> `ChooseHelp` (`ding2`, tab 5). Credits -> `ChooseCredit` (plays `tigre.wav` at 50, tab 6). Control -> `ChooseRegister` (tab 4; no sound). Video tab (`ChooseVideo`, tab 2, panels `PANOptions2`, `PANIcoVideo1-6`, `videopA..F`) is not wired to any button [DEAD].

### 4.3 Audio tab (`PANFondVolume1` keyboard / `PANFondVolume2` joypad, layer 3, `paninter.wdl:482-518`)
- Static background image: `volmus.pcx` (266x296) at **(347, 17)** (labels, drawn as an HBAR at full value): **"Głośność muzyki"** (music volume), **"Głośność dźwięków"** (sound volume), **"Rozdzielczość grafiki"** (graphics resolution); each followed by a coloured bar with "-" / "+" ends.
- Sliders, thumb bitmap `butvol1.pcx` (32x29), track length 227: music `HSLIDER (365, 60), 0..100, var Slider_MusicVol` (default 75); sound `(365, 161), 0..100, Slider_SoundVol` (default 75); resolution `(365, 264), 1..5, Slider_Video` (default 3, `skills.wdl:742-746`).
- Resolution label window: `Window (444, 300), 84x20` onto `hbarvide.pcx` (504x20 strip of six 84-px labels "320x240 512x384 640x480 800x600 1024x768 1280x960") with x offset `ShiftPanelVideo = int(Slider_Video) * 83`. Mapping `int(V) -> VideoModeChoisi`: 1 -> 5 (512x384), 2 -> 6 (640x480), 3 -> 7 (800x600), 4 -> 8 (1024x768), 5 -> 9 (1280x960) (`:250-275`); the mode is applied on close. A tick sound `clog3` at volume 30 plays when the integer value changes. In the port: map to window scale or ignore.
- Input device buttons (64x64): **Keyboard** at (400, 340) (`butkey1/2.pcx`), **Joypad** at (495, 340) (`butpad1/2.pcx`). The current device shows its "2" (lit) image; hovering the other shows its "2". Click -> `JoyPad = 0` / `1` and swaps `PanFondVolume1/2` (`options.wdl:870-880`). `JoyPad` is saved.
- Polling loop `CheckSlidersVolume` (`:214-282`, 1 tick period):
  - Music slider changed: `Midi_Vol = Slider_MusicVol`. If the audio panel is open (`AudioPanelOn == 1`), no music is playing and the left button is down: `AudioPanelOn = 2` and `Play_Song musique4, MusicVolume` (= `sammy04.mid`, preview). On close, if `AudioPanelOn == 2 && midi_playing`: `PlayCurrentMusique`.
  - Sound slider changed: `Sound_Vol = Slider_SoundVol`; if no loop is playing, `PlaySoundVolume` = `play_Loop Tigre_son, Slider_SoundVol` (`tigre.wav` looping), then after 16 ticks while the left button is held `Tune_Sound` follows the slider, then `Stop_Sound`; otherwise `Tune_Sound IdSoundLoop, Slider_SoundVol`.
  - Video slider: as above.

### 4.4 Help tab
`CurrentHelpPage` (default 1, `skills.wdl:737`). `Texte_Help`: top-left (334, 20), font `Fnt_Help` (`polish1320.pcx`, 13x20), flag Narrow, layer 5. The page picture is a panel at **(330, 297)** (297x113), layer 2. Navigation arrows `flechga1/2.pcx` (left) and `flechdr1/2.pcx` (right), 65x62, in a nav panel at (330, 14): left arrow button at panel (0, 394) = **(330, 408)**, right arrow at panel (230, 394) = **(560, 408)**; image 1 = normal, image 2 = hover/pressed [U]; sound `Ding1` on release.

| Page | Text string | Picture | Arrows |
|---|---|---|---|
| 1 | `Help1_Str` (score goal, gems, exit) | `help01.pcx`: "BIAŁY KLEJNOT" (white gem), "AKTUALNY WYNIK" (current score, example 1560), "WYNIK DO OSIĄGNIĘCIA" (score to reach, 2000) | right |
| 2 | `Help2_Str` (controls) | none (`small.pcx` 4x4) | left, right |
| 3 | `Help3_Str` (coloured doors, gem mixing) | `help03.pcx`: three colour-mix equations (red + yellow = orange, red + blue = magenta, yellow + blue = green) | left, right |
| 4 | `Help4_Str` (door types) | `help04.pcx`: "Drzwi 1", "Drzwi 2", "Drzwi 3" | left, right |
| 5 | `Help5_Str` (wasps) | `help05.pcx` (wasp types) | left, right |
| 6 | `Help6_Str` (teleports, deadly spot, restart) | `help06.pcx`: "Teleporty", "Śmiertelny punkt" (deadly point), "Wystartuj" (restart) | left, right |
| 7 | `Help7_Str` (hearts) | `help07.pcx`: "Duże serce" (big heart), "Małe serce" (small heart) | left |
Page 8 (`DisplayHelp8`, `PanHelp8`) is unreachable [DEAD]. `DisplayHelp1..7` set `CurrentHelpPage` and swap panels (`options.wdl:777-869`). Verbatim strings are in Appendix A (they contain hard line breaks, which are the intended wrapping).

### 4.5 Credits tab
`lateam.pcx` (282x294) as a non-interactive button at **(340, 25)**; `Texte_Credits` at **(334, 320)** (font `Fnt_Help`, Narrow) showing `Thanksto_Str` (Appendix A). Sound `tigre.wav` at 50 on entering the tab.

### 4.6 Registration tab (via the 4x4 Control button only) [DEAD-ish]
`LastOptionChoose = 4`: `Texte_Register` at (474, 20) centred, string `register_Str` (registered, `Singer != 0`): "This is a\nregistered\nversion of\n\nSAMMY SURICATE\nIN\nLION-LAND\n(v1.4)" plus `Texte_RegisterName` at (464, 280) = blank user name. `Button_Register`, `Texte_EnterCode` (474, 320), `Texte_MessageRegister` (474, 400) only exist in shareware builds. The code-entry flow (`EnterCode`, `DecodeKey`, `InitLvl.wdl:339-496`) is [DEAD]; the port should drop it.

### 4.7 Button semantics (in game vs menu)
| Button | In a level (`MenuOptionsGame` 1) | Extro (2) | Front-end (0) |
|---|---|---|---|
| Resume | shown. `ding2`, `Fin_MenuOptions` (continue) | same | **not shown** |
| Back | shown. `ding2`, `Fin_MenuOptions`, `waitt 1`, `Branch MainMenu` (abandon level, go to main menu) | same (leaves extro) | `Fin_MenuOptions` only (acts as "close") |
| Quit | `ding2`, `OuiNonQuit` (Yes/No, "Wyjście?") -> `QuitSammy` | same | same |
| Restart | `PanRestart`: Yes/No "Uruchomić ponownie?" -> `OuiRestartLevel` = `Fin_MenuOptions; BallesJoueur1 = 0` | not shown | not shown |
| Esc / Enter | `Fin_MenuOptions` (resume) | same | close |
`ChooseBack`'s `MainMenu` makes the still-running level thread skip win/lose (`ChoixMenu = 999`, §7.3). **Restart ends the run with zero lives**, so the Lose screen plays before the level restarts and `Facilite` goes up (§7.3).

Quirk: freezing the game (`Freeze_mode`) is released in `Fin_MenuOptions` even if the Yes/No dialog launched `QuitSammy` [irrelevant].

---

## 5. Yes/No dialog (`options.wdl:668-776`, `paninter.wdl:368-389`)
- Panel `PANYesNo` (layer 10): `fondyn.pcx` (199x68) at **(223, 135)**; Yes button `oui1/oui2.pcx` (68x28, "TAK") at panel (12, 36) = **(235, 171)**; No button `non1/non2.pcx` ("NIE") at panel (118, 36) = **(341, 171)**. Button images (same convention as 4.2): normal = `oui1.pcx` / `non1.pcx` (white "TAK" / "NIE"), hover and pressed = `oui2.pcx` / `non2.pcx` (green). Sound `Ding1` on release.
- Text `Texte_YesNo` (layer 11, font `FonteF_1216`) at (228, 138) (`SetPosPanelsOptions :586-589`). Strings: `QuitYesNo_Str` = **"Wyjście?"** ("Exit?"); `RestartYesNo_Str` = **"Uruchomić\nponownie?"** ("Restart?").
- Open (`InitChoixOuiNon`): `ResultChoixOuiNon = 0`, show panel and text, cursor to (panel.x + 100, panel.y + 20) = (323, 155), **cursor clamped** to x in [panel.x + 1, panel.x + 199 - 17], y in [panel.y + 1, panel.y + 68 - 17] (`LimitMin/MaxCursor`; in this mode the cursor moves by 2 x mouse deltas instead of following the pointer, `Mouse.wdl:16-41` [U]). Wait up to 16 ticks for all keys to be released.
- Keys: **J = yes**, **N = no** (`language == 1`; also hard-coded first, `:695-702`). Y and O are not mapped for language 1 but still exit the wait loop (`:687`) and **dismiss the dialog without any action**; Esc likewise. For the port: accept T/Y/J for Yes and N for No; Esc = cancel (no action).
- Resolve: hide panel/text, `waitt 4`, wait for mouse release, restore cursor bounds, then **Yes -> `ActionResultOui_Syn`, No -> `ActionResultNon_Syn`** (`Branch`).
- Callers:
  | Caller | Question | Yes | No |
  |---|---|---|---|
  | Menu Quit item (code 6), Options "WYJDŹ Z GRY" | "Wyjście?" | `QuitSammy` | `ActionNulle` (nothing) |
  | In-game Restart button | "Uruchomić ponownie?" | `OuiRestartLevel` | `ActionNulle` |
- `QuitSammy` (`interfac.wdl:669-820`): rebind `On_F10/On_Esc = ExitDirect`, `siger = 0`, `Save_Info "BestSco",0`, `waitt 1`, `freeze_mode = 1`; because `frenchfullversion` is defined: `waitt 4; Exit` (no end panel; the `Exit` message "Thanks you for playing Sammy" is not visible). `F10` while holding `X` calls `PreQuitSammy` (`mm1.wdl:127`, `interfac.wdl:654`), a dev shortcut.
- `ObjetMenuSelect` and `LevelNotReady` return immediately while the dialog is visible (`interfac.wdl:201`, `:1017`).

---

## 6. Mouse cursor (`Mouse.wdl`)
- Bitmap `arrow.pcx`, 16x16, a white/grey diamond (crystal) on a black (colour-key transparent) background. `MOUSE_MAP = arrow` is set on every `mouse_toggle` (`:8`).
- **Hot spot (8, 8)** = centre of the sprite (`Mouse_Spot` set in `MainMenuSammy`, `interfac.wdl:413-414`; never reset, so it also applies in options and in game) [U that the engine reads it from there].
- `mouse_toggle` (`:7-85`): `MOUSE_MODE += 2`; if it exceeds 2, set 0 and stop (so calling it twice turns the cursor off). While on, each tick: if `LimitMinCursor.x != 1` (dialog mode) `Mouse_Pos += 2 * Mickey` clamped; else `Mouse_Pos = Pointer` (system cursor). `initBoundsCursor` sets the normal limits `x in [1, Screen_width - 17]`, `y in [0, Screen_hgt - 17]` (`:88-94`).
- Visible: root menu and its sub-menus (`MainMenuSammy` turns it on and re-enables it every tick if off, `:433`); options overlay (in game it is switched on at open and off at close); Yes/No dialog. Hidden: during the intro (`MoveMenu` sets `Mouse_mode = 0`), and in levels (`GestionPartie2 :533`, `StartintroGame :650`).

---

## 7. Level flow from and to the menu

### 7.1 Save data used by the front-end (`.SAV`, see `gameplay.md` §2)
- `levelN.SAV` (N = 1..12, 17..28, 33..44, 49, 50): holds `LevelsExist[]` where only index N matters at boot (the files on disk are a copy played to level 5: levels 1-5 hold 2, 6-12, 18-28, 34-44 and 49 hold 1, 17 and 33 hold 2; `level2.SAV`..`level5.SAV` also contain the lower indices as 2, which is irrelevant). Files 13-16, 29-32, 45-48 do not exist (and the `b113..b148.wmb` in the install dir are 4756-byte stubs). Every level file has `BonusState = 0`.
- `Ver1.SAV`: `Siger = 1`, `NumberLast* = 12/28/44`, `LevelEnCours = 999`, `NeverPlayed = 0`.
- `BestSco0.SAV`: `BestScores[1..]`, `LevelsExist` of the last save, `LevelEnCours` (the "Play" target), `PreviousLevel`, `NbrLevelsReach`, `Player1Start` (checkpoint of the last level), `VideoMode*`, `LastOptionChoose`, `NeverPlayed`, sliders, `JoyPad`. (In the author's save `LevelEnCours = 5`, `NeverPlayed = 1`, levels 1-5 = 2.)

### 7.2 Starting a level
- **Play** (code 1) and **icon** (code 100+N) both end in `GestionPartie2` (`mm1.wdl:531-540`): `NeverPlayed += 1`, `Mouse_Mode = 0`, `AdaptPanels`, `Branch DebutPartie`.
- Play starts whatever `LevelEnCours` currently is: the saved value (`BestSco0`), updated after each win to the next existing, unlocked level (§7.5). The icon path overwrites `LevelEnCours` first. There is no check that the level is unlocked beyond the icon state.
- `DebutPartie`/`StartintroGame` (`mm1.wdl:542-909`) are in `gameplay.md` §1.5: black fade (`PanBlack`, fog 100 colour 4), `InitLevelGlobal`, `InitLevel<N>`, music `MusicOrder[N]`, camera sweep (skippable with Enter/Esc/Joy2), `On_Esc = InitGameOptions`, `MenuOptionsGame = 1`. The `Protection1 != 999 -> ChoixMenu = 999` check (`:719`) is anti-piracy; ignore.

### 7.3 Leaving a level
1. **Win** (`PartieGagnee = 1`, exit tile) or **lose** (`BallesJoueur1 == 0`) ends the `While (BallesJoueur1 > 0 && PartieGagnee == 0)` loop (`mm1.wdl:863-909`): enemy `IdPlayer = 0`, keys cleared, `PartieEncours = 0`, wait up to 16 ticks for keys released.
2. If `ChoixMenu != 999`: lose -> `Create <Lose.mdl>` at the camera (`InitTitleLoose`); win -> `AnimGagnee` + `Create <win.mdl>` (`InitTitleWin`). The screen runs up to **320 ticks or any key**, the sun rotating 10 degrees/tick (`:932-969`).
3. `StoreCurrentSkills` (checkpoint), best score update, `Save_Info "BestSco",0` with `Siger = 0` during the write (`:972-984`).
4. `PreviousLevel = LevelEnCours`.
5. **Lose**: `Facilite += 1`; `On_Esc = InitGameOptions`; `ResetPanels`; fog 100 colour 4; `waitt 2`; **`Branch DebutPartie`**: the **same level restarts** at once (checkpoint respected through `Player1RestartX/Y/Z`). No menu.
6. **Quit via Options Back** (`ChooseBack`): `MainMenu` sets `BallesJoueur1 = 0` and `ChoixMenu = 999`, so the level thread skips the win/lose screens (`:932`, `:962`), still stores skills and saves (`Facilite += 1` since not won), then ends at `If (ChoixMenu == 999) end` (`:1145`). `MainMenu` (`interfac.wdl:328-390`): clear key bindings, `ResetPanels`, `Camera.fog = 100`, `Fog_Color = 4`, `PanBlack` on, wait 8 ticks, wait for the current sound to finish (or Esc/Space), reload `menu_greenpepper.WMB` (retry up to 16 times), set camera to `CameraPosOrigine/AngleOrigine`, arc 70, `Sun_Light 200`, `sun_angle 250`, `MenuOptionsGame = 0`, `PLay_Song_Once Musique1`, force at least 512x384, then `GestionJeu` -> `MainMenuSammy`. The camera always starts at the **root** menu view (not a sub-menu).
7. **Restart** (Options): `BallesJoueur1 = 0` -> same as a lose (steps 2, 5): Lose screen, then the level restarts, `Facilite += 1`.
8. Pressing **R** in a level (`RecommencePartie`) also sets `BallesJoueur1 = 0` (Shift + X + R wins, dev).

### 7.4 After winning (`mm1.wdl:988-1123`)
```
Facilite = 0
if LevelEnCours < 49:
    V6 = LevelEnCours
    ValideLevel            # LevelsInfo[L] = 2 ; save levelL.SAV
    SearchNextLevel        # L = first index > L with LevelsInfo != 0 (up to 49); unchanged if none
    ValideLevel            # unlock that next level (=2) and save
    if L != V6: NbrLevelsReach++
if Siger == 1 (registered; treat as true):
    last_tut = 12 ; last_jump = 28 ; last_think = 44          # NumberLast*Level
    if (Prev == 28 && Info[28]==2 && Info[44]==2) || (Prev == 44 && Info[44]==2 && Info[28]==2):
        Branch InitExtro
    if Prev == 12 && Info[12]==2 || Prev == 28 && Info[28]==2 || Prev == 44 && Info[44]==2:
        Branch MainMenu             # root menu; LevelEnCours already points to the next level
# otherwise:
Branch DebutPartie                 # the next level starts directly (no menu)
```
(`Siger == 0` branches are [DEAD].) Notes:
- `Info[x] == 2` means "unlocked", not "beaten" (`ValideLevel` writes 2 on both the beaten level and the next one). So the Extro starts when the last Jump level and the last Think level are both *reached* and the player has just won either of them (the order does not matter) [Q].
- After world ends: `SearchNextLevel` from 12 gives **17** (13-16 absent), from 28 gives **33**, from 44 gives **49** (the Extro level has a save file). So after finishing the Tutorial world the menu's Play button points to level 17; after the Jump world to 33; after the Think world to 49, which plays the Extro level through the normal level path if reached that way (no `InitExtro` machinery) [Q].
- The player returns to the **root** menu (not a sub-menu) after the last level of a world.

### 7.5 Progression rules
- `ValideLevel` (`InitLvl.wdl:239-258`): skip only for out-of-range registered levels when `Siger == 0` (never for us); otherwise `LevelsInfo[L] = LevelsExist[L] = 2` and `Save_Info "level", L` (with `Siger = 0` written then restored).
- `SearchNextLevel` (`:259-271`): scan `V5 = L+1 .. 49` (the loop actually reads up to index 50), first `LevelsInfo[V5] != 0`; sets `LevelEnCours = V5`; if none, `LevelEnCours` unchanged.
- Net effect: every consecutive pair of existing levels unlocks the next in file order; the three worlds are independent chains starting at 1 (always 2), 17 and 33 (both stored as 2 in their files). The three first levels of each world are therefore unlocked from the start, and levels 21 and 37 get the extra rule of §3.8.
- `Facilite`: reset to 0 on a win, incremented on every lose / restart / quit-to-menu. It only feeds `VitesseBonusMin/Max = 70 / 100 - 10*Facilite` (`InitLvl.wdl:127-128`), which nothing reads: no gameplay effect [DEAD].
- `NumberLast*Level` (12 / 28 / 44 from `Ver1.SAV`; source defaults 12/36/20 in `skills.wdl:331-336`, but `Ver1.SAV` overrides).

### 7.6 Extro entry and exit (`Extro.wdl:1-153`)
Entry: `InitExtro` after the win test above (or the dev key Shift + X + E). It sets `ExtroVue = 1` (saved with `BestSco0` at the next save; it is what would unlock the Bonus item), loads `Extro.WMB` with `LevelEnCours = 49` during init, runs the same type-3 camera sweep, then sets `BallesJoueur1 = 1`, `On_Esc = InitExtroOptions`, `LevelEnCours = 20`, `MenuOptionsGame = 2`. Exit: `PartieGagnee = 1` (end trigger) -> fog colour 1 at 100, `Branch MainMenu`. `LevelEnCours = 20` means Play afterwards restarts the Jump level 20 (so the Extro is not replayed by Play) [Q]. Options Back during the Extro also goes to `MainMenu`.

---

## 8. Port checklist (most important first)

1. **Game state and saves**: `levels_info[1..49]` (0/1/2), `level_enc_cours` (Play target), `never_played`, `extro_seen`, best scores, checkpoints per level, options (music 75, sfx 75, joypad flag, last tab = 5). Initial levels: 1, 17, 33 unlocked (2); 2-12, 18-28, 34-44 and 49 locked (1); 13-16, 29-32, 45-48 absent (0). `Singer = 1`, last-of-world 12 / 28 / 44. Match the unlock rule of §7.5 (win L: mark L = 2, then next existing = 2, `level_enc_cours = next`).
2. **Menu scene** (`menu_greenpepper.WMB`): render the BSP + sky, grass, cacti, lights; camera at (1, -369, 115) pan 90 tilt 1 arc 70; 9 root items at the positions of §3.3 with the title `.wmb` models and decorative spinners; hover bob (§3.4).
3. **Picking and hover**: pointer ray picking against the clickable set; hover = `Toc2` (vol 25), orange light (255,180,50) range 300; icons scale x1.2 (1.25 -> 1.5), sparkles, tooltip following the cursor; click = `Ding1` (vol 50) + `ChoixMenu`.
4. **`ChoixMenu` state machine** (§3.6) with hard camera cuts to the 4 markers and back; Esc opens Options in every state; cursor start positions.
5. **Level icons** (§3.3 tables + §3.8 states): ghost/transparent for locked and absent, invisible for absent bonus slots, tooltip strings from §3.9, the locked message and the `Singer` special-case for 21/37.
6. **Options overlay** (§4): tabs Audio / Help (7 pages) / Credits, Resume/Back/Quit/Restart per context, sliders (music and sfx 0-100, resolution 1-5), device toggle, Polish strings from Appendix A, Esc/Enter closing, freeze the game while open, remember last tab.
7. **Yes/No dialog** (§5) with exact strings and keys (accept T/Y/J, N; Esc cancels), cursor clamp.
8. **Level exit flow** (§7.3-7.4): lose and restart replay the same level after the Lose screen (320 ticks or key); win auto-advances except after levels 12, 28, 44 (root menu) and the Extro rule; Back to menu abandons the level and returns to the root view.
9. **Intro** (§2): 3 logos with the timings of §2.2; scripted Sammy walk with the node loop of §2.3 (speed 15/tick, turn smoothing 0.25, node turn rules 0.7/0.5/0.2); follow camera; hint title; end at the first 999 node (about tick 690) or on Enter/Esc/right click; camera glide with factor 0.2 per tick; then the menu.
10. **Boot** (§1): "Sprawdzanie systemu..." text during load, music `sammy01.mid`.
11. **Cursor**: 16x16 diamond sprite, hot spot (8, 8) [U].
12. **Optional/dead**: registration, shareware/demo panels, `PanCameraChoose`, Bonus item (hidden in shipped data), video tab, resolution icon panels, help page 8, `AllLevelsOn` cheat.

---

## Appendix A: verbatim Polish strings (`SamTexts.wdl`)
Copied from the file, CP1250 decoded. Newlines inside the strings are real line breaks of the original (they define the text wrapping in the 13x20 font); trailing spaces are kept. Line numbers refer to `SamTexts.wdl`.

### `QuitYesNo_Str` (`SamTexts.wdl:6`)

```text
Wyjście?
```

### `RestartYesNo_Str` (`SamTexts.wdl:7`)

```text
Uruchomić
ponownie?
```

### `Help1_Str` (`SamTexts.wdl:12`)

```text
Sammy musi osiągnąć 
ustalony wynik by opuścić
dany poziom. Otrzymuje 
punkty za każdy klejnot,
który zbierze. W górnym
prawym rogu wyświetla się
wynik osiągnięć naszego
bohatera, oraz ilość 
zebranych przez niego 
klejnotów. Kiedy Sammy
osiągnie ustalony wynik
może opuścić poziom
i udać się do wyjścia.

```

### `Help2_Str` (`SamTexts.wdl:30`)

```text
Kierowanie:

Użyj klawiszy do
poruszania Sammy-ego.

Użyj CTRL do skoku.

Użyj SPACE do rzucania os.

Użyj SHIFT do 
przyspieszenia Sammy-ego
(musi mieć wystarczającą 
ilość zebranych-bonusów).

Użyj ALT, żeby
szybko się obrócić.

Użyj ESC, aby wrócić do
menu.


```

### `Help3_Str` (`SamTexts.wdl:53`)

```text
Kolorowe drzwi
otwierają się jeśli Sammy 
posiada klejnot z 
odpowiednim kolorem.
Sammy może nieść tylko
jeden klejnot w tym 
samym czasie.
Niektóre klejnoty mogą być 
'zmieszane'. Wynikiem może
być klejnot o nowym 
kolorze.

Właściwości mieszania:

```

### `Help4_Str` (`SamTexts.wdl:69`)

```text
-Drzwi 1 otwierają się gdy
Sammy podejdzie blisko i 
posiada pasujący klejnot.
Zamykają się gdy
znika , a Sammy trzyma
klejnot.
-Drzwi 2 otwierają się 
Sammy znika.
Sammy trzyma klejnot.
-Drzwi 3  również 
otwierają się. Ale te Drzwi
'łapią' klejnot a Sammy
traci go.
```

### `Help5_Str` (`SamTexts.wdl:85`)

```text
Sammy może łapać osy
i używać os do walki
przeciwko lwom.
Każdy rodzaj os
posiada inne właściwości.
Jedne osy posiadają 
truciznę, ale są szybkie. 
Inne mają więcej trucizny,
są powolne, ale podążają 
za lwami. Sammy może 
nosić tylko jeden rodzaj 
os jednocześnie. Kiedy
łapie osę innego rodzaju, 
traci wszystkie które miał 
poprzednio.

```

### `Help6_Str` (`SamTexts.wdl:104`)

```text
Teleporty transportują 
Sammy-ego do innych miejsc.
Musi jedynie nadepnąć na
nie. W przypadku gdy Sammy
nadepnie na śmiertelny 
punkt, traci życie i wraca 
do pozycji startowej.
Kiedy Sammy dotrze do
dużej zielonej strzały, 
będzie mógł rozpocząć grę 
z tej pozycji w której był 
tracąc życie.

```

### `Help7_Str` (`SamTexts.wdl:119`)

```text

Kiedy Sammy znajdzie duże
serce, zdobywa dodatkowe
życie.

Sammy może powiększyć swój 
status życia (lewy górny 
róg) zbierając małe serca.
```

### `Help8_Str` (`SamTexts.wdl:129`)

```text


```

### `Thanksto_Str` (`SamTexts.wdl:134`)

```text
Podziękowania dla :
Dan Silverman
(adt. textures)
Cécile, Sylvain,Héloise,
Guillaume,Aurélien,
Birgit,Manfred... 
(beta testerzy)

```

## Port status (intro, music)
- Intro: `crates/sammy/src/intro.rs`. Walker runs the §2.3 state machine per frame (rates × frame ticks), nearest-in-cone wins scans [U]; follow camera type 3 with the menu's camera zones; logos per §2.2 drive the black fog (drawn as a near-layer quad) [U: the intro loop's own fog writes are dropped]; glide per §2.6. Only a boot that lands on the menu plays it; `SAMMY_NOINTRO` or any menu debug var skips it.
- The A5 logo's background is (13,5,5), not a colour key even in RGB565, so its dark square is drawn as in the original.
- Music: `crates/sammy/src/music.rs`, MIDI rendered by `rustysynth` with a GM SoundFont (`$SAMMY_SOUNDFONT`, else first `.sf2` in `~/.local/share/sammy-suricate/` or `/usr/share/soundfonts`). Menu after a level = silence (`Play_Song_Once Musique1,0`).
