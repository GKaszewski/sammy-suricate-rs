# HUD, fonts, win/lose titles and save files (Sammy Suricate, Acknex A4 v4.22)

Deep-dive that replaces and corrects `gameplay.md` §2 (saves) and §11 (HUD). Sources are `extracted/*.wdl` (CP1250, CRLF; line numbers below are for the CRLF-stripped file, identical to the raw numbering), the PCX/MDL files in `extracted/`, and the user's real saves in `/mnt/drive/Games/sammy-suricate/drive_c/Program Files (x86)/Sammy Suricate/`.
Tags: **[U]** engine semantics not verifiable from scripts, **[I]** inference, **[Q]** quirk, **[DEAD]** unreachable. Scratch decoders: `/tmp/claude-1000/-mnt-drive-dev-sammy-rea/2bba0aa0-6655-4cb7-b593-d7d4d2d7b58a/scratchpad/hud/{pcx,sav,dump,topng}.py`.

## 0. Corrections to `gameplay.md`

| Old statement | Correction |
|---|---|
| §2 "18 `Var_Info` records `{id,count,vals}` then 38 skills `{cstring,x,y,z}`" | Right idea, but: there is a 16-byte header, **scalar** records have `count = 3` (value + 2 unused), records and skills are in **reverse declaration order**, the skill list is prefixed with a `u32` count (38), and a **187-byte string chunk** trails (§4.2). |
| §2 `LevelsExist` "1 locked, 2 unlocked" | 0 = level absent, 1 = present but locked, 2 = unlocked/validated (also set when won). `-1` = registered-only bonus (`InitLvl.wdl:246`, `interfac.wdl:1061`). |
| §2 "the user's progress is levels 1-9" | Levels 1-8 won, level 9 reached and attempted (best 390) but **not won** (`level10.SAV` is still the shipped file). See §4.6. |
| §11 `PanScore2` dead | It is the 4-tick **flash of the goal digits** when Sammy touches the exit door with too little score (`WarnScoreGoal`, `mm1.wdl:3988`, caller `:3066-3068`). |
| §11 `Black.mdl` "transitions" | Right; but it is also shown behind the options overlay with skin 2 (`options.wdl:911,1021,1053`). |
| §1.6 win: "`win.mdl` flies in" | `Call AnimGagnee` **blocks until a key is pressed** (`mm1.wdl:947`, loop `:5177`), so the "Wygrałeś" logo is created only after that key press and is removed on its first iteration while the key is still down (§3.3). |
| §11 `extrabal` / `Nextraba` popups | The two bitmaps **do not exist** in `extracted/` (`find -iname '*extrabal*'` is empty) [Q]. |
| §11 popup rise "40/tick, 32 ticks" | Speed/duration are per popup type and `VitesseFlying` is a sticky global (§1.7). |

---

## 1. HUD

### 1.1 Engine panel semantics used by the HUD
- A4 `PANEL` with `FLAGS overlay` draws the bitmap with colour (0,0,0) transparent; `transparent` blends 50 % [U]. Higher `Layer` is drawn on top [U]. Panels are **never scaled**: bitmaps are drawn 1:1 and each video mode has its own pixel positions (`initPosPanels`, `panels.wdl:806-1025`).
- `Window x,y,w,h,bmap,off_x,off_y`: draws the `w×h` sub-rectangle of `bmap` starting at `(off_x, off_y)` (variables are read every frame) at `(panel.pos + (x,y))`. All scrolling bars below rely on this [U, consistent with the art, §1.4].
- `DIGITS x,y,n,font,factor,var`: prints `var*factor` with `n` digit cells (cell advance = font glyph width). See §2.3.
- `entity ... view = CAMERA`: 3D model glued to the camera: `X` = depth ahead, `Y` = left (negative = right), `Z` = up (`panels.wdl:408-448`).
- Video modes (`interfac.wdl:1114-1196`): 2 = 320×240, 5 = 512×384, 6 = 640×480, 7 = 800×600, 8 = 1024×768, 9 = 1280×960 (3 and 4 are skipped, `panels.wdl:1204`). Menu panels are forced to 640×480 (`mm1.wdl:656-660`, `BlackScreen640 :3975`).
- `InitPanels` (`panels.wdl:640-805`) is called once per level start (`mm1.wdl:835`) after `AdaptPanels` (`:535`, = `ResetPanels` + `InitFontes` + `initPosPanels` ...). All HUD panels are shown only when `TypeJeu == 25` (set by `DebutPartie`, `mm1.wdl:544`). `ResetPanels` (`panels.wdl:1163-1189`) hides everything (called after each game, `mm1.wdl:1147`, and by the options overlay).

### 1.2 Per-mode positions (screen pixels; panel origin, add the offsets of §1.3)
Bitmap sizes: `fondlife` 88×46, `fondlif2` 50×27, `fondscor` 177×51, `fonscvga` 76×14, `fondgems` 103×89, `hbargems` 336×42, `vbarhear` 12×96, `Hbarhear` 96×8, `panboost` 43×40, `panboos2` 73×40, `Hbarboos` 300×8 (all 24-bit PCX, measured).

| Element (panel) | mode 2 (`:808-841`) | mode 5 (`:842-878`) | **mode 6** (`:880-914`) | mode 7 (`:916-952`) | mode 8 (`:953-988`) | mode 9 (`:989-1022`) |
|---|---|---|---|---|---|---|
| Lives text (`PanLifeNumber`) | -36,-18 | 2,2 | **2,2** | 4,4 | 4,4 | 4,4 |
| Life plate + hearts (`PanLife`; mode 2 `PanLife2`) | 0,0 | 12,2 | **12,2** | 16,4 | 16,4 | 16,4 |
| Score plate + digits (`PanFondScore1`/`PanScore1`/`PanScore2`; mode 2 `PanFondScoreVGA` / `PanScoreVGA`) | 236,19 / 190,0 | 330,4 | **464,4** | 616,5 | 840,5 | 1096,5 |
| Gem plate (`PanFondGems`) | not used | 204,5 | **268,5** | 348,5 | 460,5 | 582,5 |
| Gem icon (`PanGems1`) | 140,4 | 234,29 | **298,29** | 378,29 | 490,29 | 612,29 |
| Boost bar (`PanBoost`) | -12,190 | 0,322 | **0,418** | 0,538 | 0,706 | 0,898 |
| Boost dino (`PanFondBoost`) | hidden | 46,304 | **46,400** | 46,520 | 46,688 | 46,880 |
| Boost trail (`PanFondBoost2` / `3`) | 6,170 / 12,170 | 6,304 / 12,304 | **6,400 / 12,400** | 6,520 / 12,520 | 6,688 / 12,688 | 6,880 / 12,880 |
| Ammo digits (`PanNbrAmmo1`) | 236,210 | 410,350 | **540,433** | 676,542 | 900,710 | 1156,902 |

Pattern for a resolution-independent port (anchor, mode 6 values in brackets): lives top-left `(2,2)/(12,2)`; score plate top-right (`W-176`, 4); gem plate top-centre (`W/2-52`, 5); boost bottom-left (`H-62`, 0 offsets as listed); ammo digits bottom-right. Modes 7-9 differ by a few pixels (`W-184`, `H-58` ...), so **anchor and render at 640×480 then scale uniformly** (the original never scaled, so any scale is a port choice).
Quirk [Q]: `panFondAmmo1` (bitmap commented out, `panels.wdl:482-483`/`:809`) and `PanAllgems`, `PanLogoA4`, `PanFondFin*` are dead.

### 1.3 Mode 6 element detail

| Element | Bitmap / font | Panel pos + inner offset => absolute | Layer | Flags | Visible when |
|---|---|---|---|---|---|
| Life plate | `fondlife.pcx` 88×46 (`panels.wdl:6`, `:508-515`) | (12,2) | 2 | refresh,overlay | `TypeJeu==25` (`:716-721`) |
| Hearts bar | `vbarhear.pcx` 12×96, `Window -12,0,12,48,VbarHearts,0,WindowHearts` (`:513`) | window at screen (0,2), 12×48 | 2 | (same panel) | with plate |
| Lives digits | `boink22` 16×22; `DIGITS 60,20,1,...,1,BallesJoueur1` and `DIGITS 44,20,1,...,0.051,BallesJoueur1` (`:503-504`) | ones at (62,22), tens at (46,22) | 4 | refresh | with plate |
| Score plate | `fondscor.pcx` 177×51 (`:466`, `:293-299`) | (464,4) | 1 | refresh,overlay | `TypeJeu==25` |
| Score digits | `boink24` 17×24, `DIGITS 55,3,5,...,1,ScoreJoueur1` (`:281`) | (519,7), 5 cells = 85 px | 2 | refresh | with plate |
| Goal digits | `lucidd14` 15×14, `DIGITS 35,32,5,...,1,ScoreGoal` (`:282`) | (499,36), 75 px | 2 | refresh | with plate |
| Goal-flash digits | same panel layout as `PanScore1` but goal in `boink20` 14×20 (`PanScore2`, `:285-291`), pos (464,4) set at `:908-909` | (499,36) | 2 | refresh | only during `WarnScoreGoal` (§1.5) |
| Gem plate | `fondgems.pcx` 103×89 (`:468-473`) | (268,5) | 1 | refresh,overlay,**transparent** | key held (`WindowKeyPlayer1 != 0`, `:722-726`), only when `Video_Mode > 2` (`gems.wdl:340`) |
| Gem icon | `hbargems.pcx` 336×42, `Window 0,0,42,42,HbarGems1,WindowKeyPlayer1,0` (`:461-467`) | (298,29), 42×42 (centred on plate: 298+21 = 319.5) | 1 | refresh,overlay | same |
| Boost bar | `Hbarboos.pcx` 300×8, `Window 50,20,WidthBarBoost(150),8,hbarBoost,WindowBoost,0` (`:474-480`; `Define WidthBarBoost,150` `sammypolish.wdl:6`) | window at (50,438), 150×8 | 2 | refresh,overlay | `WindowBoost != 150` |
| Boost dino | `panboost.pcx` 43×40 (`:481-485`) | (46,400) | 6 | refresh,overlay | with the bar (not in mode 2) |
| Boost trail A | `panboos2.pcx` 73×40 (`:486-490`) | (6,400) | 3 | refresh,overlay,transparent | with the bar |
| Boost trail B | `panboos2.pcx` (same bitmap) (`:491-495`) | (12,400) | 4 | refresh,overlay,transparent | with the bar |
| Ammo digits | `boink22`, `DIGITS 0,0,5,...,1,NbrAmmo1` (`:499-503`... `PANNbrAmmo1` `:187-191`) | (540,433), 5 cells = 80 px | 2 | refresh,overlay | `NbrAmmo1 > 0` (shown by `InitPanelWasp3D :1402`, hidden `player.wdl:363`) |
| HUD wasp | 3D `panwasp.mdl` (§1.6) | camera space | layer 4 | | `NbrAmmo1 != 0` |
| Eye of Truth | 3D `Paneye.mdl` (§1.6) | camera space | layer 5 | | `EyeOfTruth != 0` |
| Fade | 3D `Black.mdl` (§1.6) | camera space | layer 12 | | transitions |

Mode 2 (320×240) uses `PanLife2` (`fondlif2.pcx` 50×27 at (0,0), `Window 0,27,48,8,HbarHearts,WindowHearts,0`, `:516-523`), lives digits at (-36+60,-18+20) = (24,2) and (8,2), `PanScoreVga` (`boink19` 15×20 at (225,0); goal `standard_font` 6×9 at (265,22), `:271-277`), `fonscvga.pcx` at (236,19), no gem plate, no `PanFondBoost` (`:682-689`, `:670-676`, `mm1.wdl:4382-4389`).

### 1.4 Scroll math
- **Hearts**: `WindowHearts` ranges 0..48 (`WindowHeartsOrg = 48`, `InitLvl.wdl:30-31`; `skills.wdl:785-786`). `vbarhear.pcx` rows 0-47 are black, rows 48-95 contain **4 hearts of 12 px** (measured: non-black rows 48..95). The window shows bitmap rows `[WindowHearts, WindowHearts+48)` at screen rows 2..49, so `WindowHearts=48` = 4 full hearts, `36` = 3 hearts + (hearts disappear from the top downward, a partially lost heart is clipped), `0` = empty. Hearts shown = `WindowHearts/12` (fractional). Mode 2: `Hbarhear.pcx` 96×8 non-black cols 48..95 (4 hearts of 12×8): window x offset = `WindowHearts`, so hearts disappear from the left.
  - Changes: lion hit `WindowHearts -= EnnemyResistance*HargneEnnemy` (HargneEnnemy 2) or `*0.25` (`player.wdl:567,578,660,671`, `lion.wdl:500,515,629`); lava `-= 1*time` per frame (`mm1.wdl:4895`); heart pickup `+= Skill2` (default 12), clamp 48; ignored at 48, the pickup shrinks and reappears 48 ticks later (`mm1.wdl:4738,4753-4789`). At `<= 0` -> `Player_Kaputt` (which sets `windowHearts=0`, `player.wdl:773`) -> life lost, refill to 48 at respawn (`player.wdl:896`).
- **Gem strip**: `hbargems.pcx` = 8 slots of 42 px, window offset `WindowKeyPlayer1 = KeyPlayer1 * WidthGemPanel(42)` (`mm1.wdl:93`, `gems.wdl:338`). Slot 0 is black; slots 1-7 each have 1004 non-black pixels. Key value is a colour bitmask: 1 red, 2 yellow, 3 orange (1+2), 4 blue, 5 magenta (1+4), 6 green (2+4), 7 rainbow/master. (Order confirmed by eye on the strip; mask meaning from `mm1.wdl:3047-3057`.) `KeyPlayer1` is 255 for the "all keys" state but the HUD forces slot 7 (`WindowKeyPlayer1=7*42`, `mm1.wdl:1452`, `:3884`).
- **Boost**: `Hbarboos.pcx` 300×8; colour occupies cols 0..149 (measured). Window shows cols `[WindowBoost, WindowBoost+150)` at x 50..199: `WindowBoost = 0` = full bar (150 px), `150` = empty (all black). So the bar shrinks from the right as `WindowBoost` rises.
  - Initial `WindowBoost = WidthBarBoost = 150` = **empty** (`InitLvl.wdl:69`); `BonusBoost_Hit` does `WindowBoost -= Skill2` clamped at 0 (`mm1.wdl:4428-4432`), refused (pickup bounces) if already 0 (`:4393`); cheat Shift+O sets 0 (`mm1.wdl:5105-5108`).
  - Drain: Shift/Joy_3 held and `WindowBoost < 150` -> `LessBoost`: `WindowBoost += ConsoBoost*time` (`ConsoBoost = 1` per tick, `InitLvl.wdl:22`), so a full bar lasts 150 ticks = 9.4 s [I, 16 ticks/s]; at `>= 150` clamp and hide all four boost panels (`player.wdl:1799-1815`; same in the extro `Extro.wdl:307-323`).
  - Show on pickup: `PanBoost`, `PanFondBoost2/3` visible; `PanFondBoost` visible except mode 2 (`mm1.wdl:4377-4390`).

### 1.5 Per-tick HUD updates (main loop `mm1.wdl:863-909`, `While (BallesJoueur1 > 0 && PartieGagnee == 0)`)
- **Goal blink**: if `ScoreJoueur1 >= ScoreGoal && PanFondOptions.visible == 0` and `ticks1 % 12 == 0`: toggle `PanGems1.visible` (`:871-885`). `ticks1` is incremented **once per rendered frame** (`:893`) and zeroed whenever the 64-frame fps average completes (`:899-905`), so the half-period is 12 frames, not 12 ticks [I]. At the moment the goal is reached `AddPoints` plays `ScoreReach_Son` (applause.wav, vol 35), sets the player's key to 255 (no more gem pickups: `playerOldKey = 999`) and forces the **rainbow** gem icon and plate (`mm1.wdl:1445-1458`); the icon then blinks until the level ends. The plate `PanFondGems` stays visible, only the icon toggles. Port: toggle every ~4 ticks (12 frames at ~50 fps = 0.24 s) [I].
- **Wasp spin**: `If (nbrAmmo1 > 0) panwasp.pan += 2*time` (`:887-890`): 2° per tick (32°/s) around the model's yaw.
- **Exit-door score warning**: touching an exit door (`Skill6 == 7`) with `ScoreJoueur1 < ScoreGoal` calls `WarnScoreGoal` (`mm1.wdl:3066-3068`): guarded by `ClignoteScore`; hides `PanScore1`, shows `PanScore2`, `waitt 4`, swaps back, `waitt 4`, clears the guard (`:3988-4005`; mode 2 uses `PanScoreVga`/`PanScoreVga2`, `:4006-4020`). So an 8-tick cycle where the goal number is drawn in the larger `boink20` font for 4 ticks. No text message exists.
- **Score/lives/ammo digits** are live `DIGITS` bindings; `AddPoints` (`mm1.wdl:1440-1610`) changes `ScoreJoueur1`. Extra life before the goal: if the new score crosses `ExtraBalle1..8` (`InitExtraBalles` object, `:1611-1622`, default 3000) and `BallesJoueur1 < MaxBalles (19)`, `BallesJoueur1 += 1`, `Gagne1` sound, popup `extrabal.pcx` (`:1545-1608`). After the goal: crossing `NextraBall1..8` (default 3000) does `NbrNextraBall += 1`, popup `Nextraba.pcx` (`:1461-1538`). `MaxBalles = 19` (`skills.wdl:369`), so one tens digit (0/1) suffices.
- **Lose a life**: `Animmoins1life` creates `lifeA.mdl` running `FlyingHearth` (`player.wdl:760-771,994-1040`): a heart model (370 verts, 11 frames, 32×32 skin) at Sammy, scale ×0.7, moves 15 quants/tick along the inverse camera direction (+22° tilt, ±2° jitter), spins 30°/tick, emits `particle_life` every tick (docs/particles.md §2.16), plays `Heart_Away` (haway.wav) after 2 ticks, lives `DureeFlying = (CameraAngle.z/20)*4` ticks.
- `InitPanels` itself re-shows the boost panels when `WindowBoost != 150` and the eye when `EyeOfTruth != 0` (`panels.wdl:646-649,683-689`), so a mode switch mid-level keeps them.

### 1.6 3D HUD models (all `view = CAMERA`, `panels.wdl:408-448`)
Skins are 1-based. Frame 1 (the `base` pose) is shown; the HUD never calls `set_cycle`.

| Entity | Model | Layer | Declared X,Y,Z | Set at show-time | Other |
|---|---|---|---|---|---|
| `PanWasp` (`:408-420`, `InitPanelWasp3D :1397-1410`) | `panwasp.mdl` (MDL3, 62 verts, 96 tris, 155 frames, **7 skins 92×64**) | 4 | 100,-50,0 -> **100,-45,-35** (`:1405-1406`) | `pan = 90`, `overlay = 1`, `ambient = 80` (`:1400`); `albedo = 50` declared | spins `pan += 2*time` |
| `PanEye` (`:422-435`) | `Paneye.mdl` (flat quad 56.4×19.2 quants, 1 skin 128×35: red/yellow eyes) | 5 | **200,-45,-72** | `ambient = 100`, `albedo = 0`; additive `Flare = 1` if `Video_Depth > 8`, else `Transparent = 1` (`mm1.wdl:5028-5035`) | scale Y/Z shrink |
| `PanBlack` (`:436-448`) | `Black.mdl` (flat quad 55.6×39.8, **2 skins 16×64**: skin 1 black, skin 2 brown gradient) | 12 | **20,0,0** | `ambient = 0`, `albedo = 0`; `Skin=1` for level fades (`mm1.wdl:550-551`), `Skin=2` behind the options overlay | at 20 quants ahead it overfills the view |

- **Wasp skins/scales** (`InitCurrentPanWasp`, `panels.wdl:1411-1461`): skin = `WaspCurrentType` (1..7), uniform scale 1: 0.3, 2: 0.23, 3: 0.2, 4: 0.2, 5: 0.25, 6: 0.3, 7: 0.2. Skin tints (average colour of the skins): 1 orange-yellow, 2 dark brown, 3 blue, 4 green, 5 magenta, 6 beige, 7 grey-white. Re-applied when the type changes (`bullets.wdl:139`); `WaspCurrentType` set by `Ammo1_Hit` (`bullets.wdl:121-145`), cleared with the last shot (`player.wdl:360-368`). Hidden by `ResetPanels`.
- **Screen position check** [I]: A4 `camera.arc = 60` (`mm1.wdl:750`) is the horizontal FOV. Projection `sx = W/2 - (Y/X)*(W/2)/tan(30°)`, `sy = H/2 - (Z/X)*(W/2)/tan(30°)`: wasp -> (569, 434) at 640×480, close to the commented 2-D replacement at (580,428) (`panels.wdl:886-887`); eye -> (445, 440) (bottom, right of centre). Port: place a billboard at those pixels or keep real camera-space entities.
- **Eye of Truth lifetime** (`EyeOfTruth_Hit`/`ReduitEyePanel`, `mm1.wdl:5018-5104`): pickup sets `EyeOfTruth = 1`, shows the panel at scale 1, `DureeEyeTruth = DureeEyeTruthOrg = 256` (`InitLvl.wdl:6`). While `DureeEyeTruth > 0` it counts down by `time`; afterwards `PanEye.scale_Z` and `scale_Y` shrink by `0.005*time` per frame (196 ticks from 1.0 to 0.02), then hide, reset scales to 1, `X = 200`, `EyeOfTruth = 0`. Total ~452 ticks (28 s). Picking another eye resets scale and timer. Death: `scale_Z = 0.01`, hidden (`player.wdl:780-782`); respawn restores scale 1 (`:990-991`). The eye also switches on the lions' attack-range rings (`lion.wdl:1591,1660`).
- **Black fade**: `PanBlack.visible = 1` at `DebutPartie` start (`mm1.wdl:550`), hidden after the pre-roll in `StartintroGame` (`:670`); also on menu/protection/extro transitions (`interfac.wdl:342,409`, `mm1.wdl:2009`, `Extro.wdl:9,26`).

### 1.7 Score and bonus popups (billboard sprites, not panels)
`FlyingPCX` (`mm1.wdl:1333-1384`): passable, `ambient 100`, `facing 1`. Spawns at the pickup (`model_pos` = object x,y, z+100 or +32), `pan = InvDirCamera.pan`, `tilt = InvDirCamera.tilt + 20 + rnd(-1,1)` (the pan jitter line is overwritten, `:1350-1352`). Each frame it moves `VitesseFlying*time` along its own x axis (toward the camera and up), advances `Frame += 0.2*time` over `FrameFlying` frames (frames are laid **side by side**: `plus50+6.pcx` 180×12 = 6 frames of 30×12 in six colours; `plu100+2.pcx` 64×16 = 2 frames of 32×16, red/yellow), and dies after `DureeFlying` ticks. `AlphaFlying` > 100 means alpha = `AlphaFlying - 100` (`:1344-1348`).

| Popup | Trigger | Bitmap | `DureeFlying` / frames | Speed |
|---|---|---|---|---|
| +50 | `Bonus3_Hit` diamond (`:1252-1263`) | `plus50+6.pcx` | 32 / 6 | 40, alpha 200 |
| +100 | `Bonus3_Hit` (`:1264-1275`) | `plu100+2.pcx` | 48 / 2 | 40, alpha 200 |
| +100 | `BonusGem` (`gems.wdl:479-493`) | `plu100+2.pcx` | 16 / 2 | current `VitesseFlying` |
| +50/+100 | `Bonus5_Hit` (`mm1.wdl:2077-2108`) | both | 16 / 6 or 2 | current `VitesseFlying` |
| extra life | `AddPoints` (`:1597-1608`) | `extrabal.pcx` (missing) | 32 / 1 | current |
| next-level extra life | `AddPoints` (`:1530-1538`) | `Nextraba.pcx` (missing) | 32 / 1 | current |

[Q] `VitesseFlying` defaults to 20 (`skills.wdl:37`) but `Bonus3_Hit` sets it to 40 permanently, so popups from gems/extra-lives are slower until the first diamond of a session. `FlyingLimitedPCX` (`:1294`) and `ActionpointsBalle1/2` (`:1387-1432`) are Ploing legacy for balls: [DEAD] for Sammy. Missing bitmaps: the engine `Create` of a missing file presumably fails silently [U]; port should draw the extra-life popup anyway (suggest a heart or "+1" sprite).

### 1.8 In-game / HUD-adjacent texts
There are **no in-level messages** (no "not enough score", no "hit R for next level": `Texte_NextLevel`, `Texte_GameOver`, `Texte_PRET`, `Texte_GO`, `Texte_Level/Titre_Level/SousTitre_Level`, `PANScoreGoal`, `BestScore` are all disabled: `InitPanelsPret` call is commented, `mm1.wdl:677-716,877,936,946,1448`; `panels.wdl:1124-1131` is [DEAD]). Level titles exist only as menu tooltips.

| Text object | Where | Font | Position | String (exact) |
|---|---|---|---|---|
| `Texte_CheckingSystem` | boot (`sammypolish.wdl:229,590`) | `Fnt_Help` (polish1320) | 4,24 | `Sprawdzanie systemu...` (`SamTexts.wdl:215`) |
| `Texte_Warning1` | boot, if `Video_Depth==16 && AverageFps<10`, until key or 128 ticks (`sammypolish.wdl:580-588`) | `FonteF_1216` (polish1017) | x = `DemiEcranX` (centred), y=20 | `\nTHE GAME SEEMS TO BE VERY SLOW,\nIT IS ADVISABLE TO QUIT (F10),\nAND START IT IN SOFTWARE MODE.\n` (`SamTexts.wdl:204-209`) |
| `Texte_NOLEVEL` | boot if no level found, 32 ticks (`sammypolish.wdl:385-392`) | `Fonte_grostitre` (lucida38), centred x,y at screen centre | | `WARNING\nI FOUND\nNO LEVEL` (`SamTexts.wdl:195-197`) |
| `Texte_PublicWrong` | protection failure, 64 ticks then quit (`mm1.wdl:2014-2021`) | lucida38 centred | | `Please Register.\nwww.suricate-software.com` (`SamTexts.wdl:199-200`) |
| `Texte_Version` | menu (`interfac.wdl:416`) | `standard_font` (ackfont) | 2,4 | `Sammy Suricate V1.42` (`SamTexts.wdl:193`) |
| `Texte_LevelNotReady` | level-icon hover, menu (`interfac.wdl:1016-1081`) | `FonteF_1216` | mouse-64, mouse+26 | by state (below) |

LevelNotReady strings (`panels.wdl:219-232`, selection `interfac.wdl:1019-1066`, `V = LevelsInfo[icon-100]`, `singer` = registration flag):
- `mustbewin_Str` = `Musisz zakończyć poprzedni\npoziom przed rozpoczęciem\nnastępnego poziomu` (V==1, or V==1 with `singer==1`).
- `LevelNothere_Str` = `Go to http://www.suricate-software.com\nto download this level` (V==0 with `singer==1`, or V==0 demo levels).
- `LevelNothereReg_Str` = `Go to http://www.suricate-software.com\nto download this level\n(For registered users only)` (V==0, `singer==0`, full-version level).
- `OnlyReg_Str` = `For registered users only` (V==1 or 2 or -1 with `singer==0`).
Level tooltips (`level<N>_Str`, `panels.wdl:94-175`): 1-12 `Szkolenie <N> :`; 13-16 `Bonus 1..4`; 17-28 `Ćwicz zręczność 1..12`; 29-32 `Bonus 5..8`; 33-44 `Ćwicz myślenie 1..12`; 45-48 `Bonus 9..12`; 49 `EXTRO`; `Titre<N>_Str` are empty; only `SousTitre44_Str` = `Hint : Yellow is the key` is non-blank ([DEAD], used only by `InitPanelsPret`). In the Polish build (shipped `Ver1.SAV`: `NumberLast* = 12/44/28`) all 12+12+12 levels are open.
Unused strings kept for completeness: `PartiePerdue_Str "GAME OVER"`, `PartieGagnee_Str "YOU WIN !!!"`, `Pret_Str "Hit a Key to start"`, `GO_Str "LET'S GO !"`, `ScoreGoal_Str "SCORE TO REACH          "`, `BestScore_Str "BEST SCORE "`, `NextLevel_Str "Hit 'R' to go to next Level"`.

---

## 2. Bitmap fonts

### 2.1 Declarations (`fontes.wdl`, `SamTexts.wdl:2-3`) and measured layout
`FONT name,<pcx>,cell_w,cell_h`. All PCX are 24-bit; measured sizes:

| Font name | PCX | Cell | PCX size | Layout (measured) | Used by |
|---|---|---|---|---|---|
| `standard_font` (`fontes.wdl:1`) | `ackfont.pcx` | 6×9 | 192×72 | 32 cols × 8 rows = 256 glyphs, code = row*32+col; ASCII 33..127 present (plus CP437-like extras) | `Texte_Version`, VGA goal `DIGITS`, debug HUD (`debug.wdl:14-31`) |
| `Fonte_grostitre` (`:2`) | `lucida38.pcx` | 25×38 | 800×304 | 32×8 = 256; glyphs at codes 33..126 and 161..254 (rows 1-3 and 5-7; rows 0 and 4 blank). High half = the same letters with CP1252 accents substituted (ä è é ê ë ï ö ü ...) | NOLEVEL, PublicWrong, dead Level texts |
| `Fonte_messagescore` (`:3`) | `boink38.pcx` | 24×38 | 768×304 | 32×8 = 256; upper-case only (lower-case slots show capitals), glyphs 33..123 and 161..254 | dead Pret/BestScore/ScoreGoal texts |
| `fntnbr_life` (`:11`) | `boink22.pcx` | 16×22 | 176×22 | **11 glyphs in 1 row**: `0123456789:` | lives digits, ammo digits |
| `fntnbr_reach` (`:13`) | `lucidd14.pcx` | 15×14 | 165×14 | 11 glyphs `0-9:` (italic, white) | goal digits |
| `fntnbr_reach2` (`:14`) | `boink20.pcx` | 14×20 | 154×20 | 11 glyphs `0-9:` | goal digits when flashing |
| `fntnbr_score` (`:16`) | `boink24.pcx` | 17×24 | 187×24 | 11 glyphs `0-9:` (yellow, red outline) | score digits |
| `fntnbr_score2` (`:17`) | `lucida16.pcx` | 18×16 | 198×16 | 11 glyphs `0-9:` | **unused** |
| `fntnbr_scoreVGA` (`:19`) | `boink19.pcx` | 15×20 | 165×20 | 11 glyphs `0-9:` | mode-2 score |
| `Fnt_Help` (`SamTexts.wdl:2`) | `polish1320.pcx` | 13×20 | 416×160 | 32×8 = 256, **CP1250**: rows 0 blank (control codes), row 1 = 32..63, row 2 = 64..95, row 3 = 96..127 (127 blank), rows 4-7 = 128..255 in CP1250 order (Ś Ť Ž Ź, Ł Ą Ş Ż, ...) | help pages, credits, register/name fields, "Sprawdzanie systemu..." |
| `FonteF_1216` (`SamTexts.wdl:3`) | `polish1017.pcx` | 10×17 | 320×136 | same CP1250 layout | tooltips, warnings, yes/no |

Rule for the 256-glyph fonts: **glyph index = byte value** of the CP1250 string byte (WDL strings are CP1250), cell column = `idx % 32`, row = `idx / 32`; blank cells are real glyphs (space). Glyph (0,0,0) = transparent [U]. Italic/outlined glyphs have padding inside the cell; **fixed-width cells**, advance = cell width, no kerning.
Font names in the old docs (`lucida38` 25×38 etc.) are right; the previous summary missed the glyph layout.

### 2.2 Digit fonts
`boink22/lucidd14/boink20/boink24/lucida16/boink19` have exactly `11 × cell_w` pixels width and one row: glyphs `0..9` then `:`. They contain no space or minus glyph [measured], so `DIGITS` with these fonts can only print digits. Engine mapping (glyph index = digit value for 11-glyph bitmaps, index = ASCII for 256-glyph ones such as `standard_font` in `debug.wdl`) is [U] but forced by the art.

### 2.3 How `DIGITS x,y,n,font,factor,var` renders [U]/[I]
- Text = `var * factor` converted to an integer. `n` cells, cell advance = glyph width, drawn left to right from `(x,y)`.
- Because the fonts have no blank glyph, the number is printed **zero-padded** to `n` digits (score 17 -> `00017`, ammo 3 -> `00003`) [I; confirm in the wine build].
- Overflow keeps the **low `n` digits** [I]: the lives display puts a 1-cell ones digit at x+60 and a 1-cell tens digit at x+44 (16 px apart = glyph width), so a 2-digit value must clip to 1 digit per panel row.
- `factor`: the lives tens digit uses `0.051` with `n=1`. For `BallesJoueur1 <= 19`: `v*0.051` is 0.05-0.97, so with rounding to nearest it gives 0 for 0-9 and 1 for 10-19 (`MaxBalles = 19`, `skills.wdl:369`). Reimplement as `lives / 10` (tens, shown as `0`/`1`) and `lives % 10` (ones) [I]. Whether a leading tens `0` is drawn is [U]; the zero-pad rule above says it is (so lives 5 shows `05`).
- `ScoreJoueur1` and `ScoreGoal` are integers < 100 000 (5 cells).

---

## 3. Win and lose title screens

### 3.1 Flow (`mm1.wdl:910-1156`)
1. `PartieEncours = 0`, input handlers cleared (`:912-917`). Wait (max 16 ticks) for any held key to be released (`:925-930`).
2. If `ChoixMenu != 999` (not an abort to menu):
   - **Lose**: `Vec_set(model_pos, Camera.pos)`, `Create <Lose.mdl>, model_pos, InitTitleLoose` (`:937-942`). `sammypolish.wdl:10` defines `tgawinlose`, so the `Ifndef` takes the `ifelse` branch (`Lose.mdl`, not `Loose.mdl`).
   - **Win**: `Call AnimGagnee` (**blocks**), then `Create <win.mdl>` with `InitTitleWin` (`:947-950`).
3. `Timer = 320`: `While (key_any == 0 && timer > 0 && ChoixMenu != 999)`: `Timer -= time`, `Sun_Angle.pan += 10*time` (`:961-969`). So the screen lasts **320 ticks (20 s) or the first key**, with the sun spinning 10° per tick.
4. Then `StoreCurrentSkills`, best score, `Save_Info "BestSco"` (§4), next level or menu (§4.5).

### 3.2 Models
`Win.MDL` and `Lose.mdl` are **MDL5 flat quads (6 verts, 4 tris, 1 frame)** with a single ARGB4444 skin of the Polish words: `Win.MDL` skin 256×88 reads **"Wygrałeś"** (cyan-silver 3D lettering), `Lose.mdl` skin 256×81 reads **"Przegrałeś"** (gold lettering). Local quad extents: Win y ±132, z -43.9..48.3, x = -72.1; Lose y ±139, z -41.2..45.5, x = -72.1. So the visible logo is 264×92 (Win) / 278×87 (Lose) model units, ×0.32 scale = **84.7×29.5** / 89×27.8 quants. `GAME OVER` / `YOU WIN !!!` strings are not used.

### 3.3 `InitTitleWin` (`mm1.wdl:5203-5309`) and `AnimGagnee` (`:5172-5202`)
- `AnimGagnee` (called in the same coroutine, blocks): Sammy (`Player_Syn`): over 10 ticks blends into the `Iwin` animation (`PlayerWin_Str = "Iwin"`, `:81`) by `lionAnimPreviousStep += 10*time` (to 100), then loops `Iwin` at `+6*time` percent per tick (16.7 ticks per loop) while `Pan += 12*time` (12° per tick = 192°/s at 16 ticks/s, one turn per 1.9 s). The loop ends only on `Key_Any != 0`. The `Win_Son` (Iwin.wav, range 2000) was already played at the exit tile (`:3969`). The camera keeps following (nothing else moves).
- Logo (`InitTitleWin`): `passable`, `Near = 1`, `Ambient 0`, `Albedo 0`, `Metal 1`; under `tgawinlose`: `Ambient = 100`, `Facing = 1` (billboard), (`Scale 1.2` is immediately overwritten) (`:5206-5215`). **Scale 0.32 on all axes** (`:5221-5223`). `Skill1 = 0` depth, `Skill2 = 16` speed, `Skill6 = 255` (unused).
- Phase 1 `While (Skill1 < 128 && key_any == 0)`: position `= Camera.pos + R(Camera.pan,tilt,roll) * (Skill1, 0, 20)` [I: `VecRotate v, Camera.pan` passes the camera angle triple, as the comment on `DevantCam` `panels.wdl:1385` says], i.e. **camera-relative: `Skill1` ahead, 20 units above the optical axis**. Then `Skill1 += Skill2*time`, `Skill2 -= 1*time` down to a floor of 2. With `time = 1`: depth 16, 31, 45, 58, 70, 81, 91, 100, 108, 115, 121, 126, **130** (13 ticks), then stays at 130. Orientation `pan = Camera.pan-180`, `tilt = -Camera.tilt` (face the camera; moot with `Facing`). `Ent_cycle(Stand_str)` is a no-op (1-frame model). Particles: `V = int(random(172)); Ent_Vertex(My_pos, V)` (the quad has only 6 verts so most indices are out of range -> particles emit at the logo centre [U/Q]); under `tgawinlose` `My_pos` is then recomputed at `(Skill1, 0, 70)` relative to the camera (`:5261-5267`), i.e. **50 units above the logo centre**; then `Particle_Win` (`particul.wdl:2963-2992`; see `particles.md` §3.3: `AverageFps > 12`: every 3rd frame one star of each of 6 colours, else every 6th frame one dot of each colour).
- Phase 2 `While (key_any == 0)`: same placement, `PlayerAnimStep += 3*time` (no visual effect), particles every frame; ends on key; `Remove my` (`:5273-5308`).
- [Q] Because `AnimGagnee` only returns after a key press, and phase 1/2 loops are guarded by `key_any == 0`, the logo entity starts while that key is still held and removes itself on its first iteration; the following `Timer = 320` loop is also skipped by the held key. **In practice the "Wygrałeś" logo is almost never seen** and the win screen is Sammy dancing until a key press [I, unverified in-game]. Faithful port: dance (Iwin loop, 12°/tick) until key, then proceed; optional enhancement: show the logo.
- Sounds: none inside the title actions; applause (`ScoreReach_Son`) played when the goal was reached, `Win_Son` at the exit.

### 3.4 `InitTitleLoose` (`mm1.wdl:5314-5393`)
- `passable`, `Near = 1`, `Ambient 90`, point light range 500 colour (250,200,50) amber (`:5317-5322`), scale 0.32.
- Phase 1 (`Skill1 < 128 && key_any == 0`): speed starts at 20 and decays 0.8/tick (floor 2): depth 20, 39.2, 57.6, 75.2, 92, 108, 123.2, **137.6** (8 ticks), same camera-relative placement `(Skill1, 0, 20)` and `pan = Camera.pan-180`, `tilt = -Camera.tilt` (no `Facing` here, so orientation matters), plus `emit 1,My_pos,particle_TileDeath` every tick from a random vertex (same out-of-range caveat; here `My_pos` is the placement point, z=20 units above axis).
- Phase 2 (`While key_any == 0`, `:5358-5390`): stays at depth 137.6, `Set_cycle LionStand_str` ping-pong `PlayerAnimStep 0..96` (no-op on a quad), 1 `particle_TileDeath` per tick (docs/particles.md §3.4 smoke). Ends on key; the Lose screen **does** last up to 320 ticks because no earlier key is held (step 1 waits for key release).
- No sounds in the actions.

### 3.5 What happens after (`mm1.wdl:972-1156`)
`StoreCurrentSkills` -> best score -> `Save_Info "BestSco",0` (with `Siger=0`) -> `PreviousLevel = LevelEncours` -> on win: `Facilite=0`, `ValideLevel`, `SearchNextLevel`, `ValideLevel`, `NbrLevelsReach++` (when a next level exists); on loss `Facilite += 1` and the same level restarts (`Branch DebutPartie`). End-of-world branches: see `gameplay.md` §1.6.

---

## 4. Save files

### 4.1 Which files and when
Files live next to the exe (`name` + number + `.SAV`): `Save_Info "level",N` -> `levelN.SAV`, `"Ver",1` -> `Ver1.SAV`, `"BestSco",0` -> `BestSco0.SAV`. `Save_Info` writes the **entire** set (18 `Var_Info` + 38 `Type Global` skills + 3 string slots); `Load_Info` overwrites the whole set. Loading is asynchronous (the script waits for the sentinel `LevelEnCours == 999`, `sammypolish.wdl:291,297-307`) [I].

| Write | Where | Notes |
|---|---|---|
| `levelN.SAV` | `ValideLevel` (`InitLvl.wdl:239-258`): `LevelsInfo[N]=2; LevelsExist[N]=2; Siger=0; Save_Info "level",N; Siger=Singer` | Called (a) at boot for the first existing level (`sammypolish.wdl:349-354`, this is why `level1.SAV` is rewritten on every start, mtime 2026-10-11), (b) after a win for the level just won and for the next level (`mm1.wdl:993-996`). Early exit for unregistered players on registered-only levels (`InitLvl.wdl:243-252`). The snapshot contains stale copies of everything else. |
| `BestSco0.SAV` | after **every** game, win or loss, unless aborted to the menu (`mm1.wdl:972-982`); `QuitSammy` (`interfac.wdl:669-681`); registration (`InitLvl.wdl:478`); `MakeSAV` | the real "profile". `Siger = 0` while writing, so the file never carries registration. |
| `Ver1.SAV` | registration `EnterNreg`/`DecodeKey` (`InitLvl.wdl:339-387,420-496`), dev `MakeSAV` (`sammypolish.wdl:604-708`) | carries `siger`, `NumberLast*Org`, `RegistrationTypeOrg`, `UserName[]`, `LevelEnCours = 999`. The shipped file has `siger = 1`. |
| `level50.SAV` | dev leftover (`LevelEnCours 50`, older `NumberLast* = 12/36/20`, 51-byte tail) | not read (`NbrLevels = 49`) |
| `Ver0.sav` | not a Save_Info file | launcher text, ignore |

### 4.2 Container and payload layout (verified on 41 files)
```
file:  u32 BE compressed_len   (= file_size - 8)
       u32 BE uncompressed_len (2656; 2520 for level50.SAV)
       LZSS data               (crates/formats/src/lzss.rs; first flag byte is 0xFF)
payload (all little-endian):
  0x00  u32 0x0000070C   constant in every file  [U: engine magic/size]
  0x04  u32 0x00000016   constant                [U]
  0x08  u32 0x03000134   constant                [U]
  0x0C  u32 record_count = 18
  0x10  18 x record { u32 id, u32 count, i32 value[count] }   (reverse declaration order)
  0x59C u32 skill_count = 38
        38 x skill { cstring name, i32 x, i32 y, i32 z }      (reverse declaration order)
  0x9A5 string chunk (187 bytes; 51 in level50):
        u32 0x2625, u32 len (167; 31),
        records { u32 (0x04000000|string_id), NUL-terminated text }:
          id 0xD8 -> 126 spaces, id 0x16 -> "" , id 0x0F -> 26 spaces (= UserNameAlone_Str, `skills.wdl:819`)
        trailer: 00, u32 0x2625, u32 0, u32 0x2625
```
Sizes: 0x9A5 + 187 = 2656. All string contents are blank in every shipped/user file, so the chunk is safe to skip on import and to zero on export [I].
Every skill's `x` is the skill value; `y`,`z` are 0 except `Player1Start` (a 3-vector).

### 4.3 Fixed point
Acknex A4 `var` = signed 32-bit **22.10 fixed point**: stored `i32 = round(value * 1024)`; scalars e.g. `LevelEnCours 9` -> `9216`, `BestScores[1] 540` -> `552960`, `Player1Start.x 2790.501` -> `2857473`. Resolution 1/1024. Array records hold `count` elements (50 or 33) with index 0..49 (levels are 1-based: element 0 unused); scalar records have `count = 3` (`value, 0, 0` [I]).

### 4.4 Variable map
IDs are the engine's global var indices; list is in file order. Declarations from `skills.wdl` (line) unless noted.

| # | id | Name (WDL) | Decl. | Elements | Meaning / who uses it | BestSco0 value (user) |
|---|---|---|---|---|---|---|
| 1 | 0x134 | `JoyPad` | 841 | 1 | 0 keyboard, 1 joypad (`options.wdl:871,876`) | 0 |
| 2 | 0x130 | `ExtroVue` | 836 | 1 | extro seen (`Extro.wdl:15`); with `BonusActif==1` enables the bonus-menu entry (`interfac.wdl:50`) | 0 |
| 3 | 0x12F | `RegistrationTypeOrg` | 834 | 1 | 0 registered by .SAV, 1 by typed code | 0 |
| 4 | 0x12C | `UserName` | 802 | 33 | registered name as ASCII codes; default `1..32,0` is the "empty" placeholder (copied to `CurrentUserName`, `sammypolish.wdl:313-320`) | 1..32,0 |
| 5 | 0x12B | `slider_musicvol` | 746 | 1 | 0..100 -> `Midi_Vol` (`sammypolish.wdl:398`) | 91.26 |
| 6 | 0x12A | `slider_soundvol` | 745 | 1 | 0..100 -> `Sound_Vol` (`:397`) | 100 |
| 7 | 0x129 | `ShiftPanelVideo` | 743 | 1 | pixel shift of the video-choice bar in options (`paninter.wdl:493`); default 332 | 415 |
| 8 | 0x128 | `slider_video` | 742 | 1 | 0..5 = modes 2,5,6,7,8,9 (`panels.wdl:1272-1297`); default 3 | 5 (mode 9) |
| 9 | 0x124 | `LevelsExist` | 523 | 50 | per-level state 0/1/2 (-1), index = level | [1..9]=2, rest 0 |
| 10 | 0x121 | `BonusState` | 338 | 1 | 2 in bonus-level files made by `MakeSAV` (`sammypolish.wdl:636-641`); copied to `BonusActif` (`:277-280`) | 0 |
| 11 | 0x120 | `NumberLastJumpLevelOrg` | 336 | 1 | last open "jump" level (default 20, shipped 28) | 28 |
| 12 | 0x11F | `NumberLastThinkLevelOrg` | 335 | 1 | last open "think" level (36 -> shipped 44) | 44 |
| 13 | 0x11E | `NumberLastTutLevelOrg` | 334 | 1 | last open tutorial (12) | 12 |
| 14 | 0x119 | `BestScores` | 325 | 50 | best score per level (`StoreBestScore`, `InitLvl.wdl:287-291`; read `:231`) | see §4.6 |
| 15 | 0x118 | `Player1RestartZ` | 314 | 50 | checkpoint z per level (0 = none) | all 0 |
| 16 | 0x117 | `Player1RestartY` | 313 | 50 | checkpoint y | all 0 |
| 17 | 0x116 | `Player1RestartX` | 312 | 50 | checkpoint x; level start uses it if x != 0 (`InitLvl.wdl:232-237`); written by `StoreCurrentSkills` (`:272-286`): stores `Player1Start` if `DoRestart`, else zeros | all 0 |
| 18 | 0x115 | `siger` | 4 | 1 | registration flag; **only `Ver1.SAV` carries 1** (other files are written with `Siger = 0`); runtime copy `Singer` (`skills.wdl:801`) | 0 |

Skills (`Type Global`), file order:

| # | Name | Decl. | Meaning | BestSco0 | Ver1 |
|---|---|---|---|---|---|
| 1 | `LastOptionChoose` | 733 | last options tab (1 audio, 2 video, 3 register, 5 help, 6 credits) | 1 | 5 |
| 2 | `Averagefps` | 437 | measured fps average, drives auto video-mode drop and `Tempo_LoadLevel` (`mm1.wdl:723`) | 61.219 | 64.678 |
| 3 | `EndJumpMessage` | 348 | end-of-world message seen (dead code) | 0 | 0 |
| 4 | `EndThinkMessage` | 347 | same | 0 | 0 |
| 5 | `EndTutMessage` | 346 | same | 0 | 0 |
| 6 | `PreviousLevel` | 345 | level just played (`mm1.wdl:986`) | 9 | 1 |
| 7 | `NbrLevelsReach` | 344 | count of levels advanced (`mm1.wdl:997`) | 8 | 0 |
| 8 | `LevelEnCours` | 343 | current/resume level; **999** in `Ver1.SAV` = load sentinel | 9 | 999 |
| 9 | `Player1Start` | 311 | current start position (x,y,z; fixed point) | (2790.501, -1073.701, 229.894) | same |
| 10 | `VideoDepthBak` | 309 | video depth | 16 | 0 |
| 11 | `VideoModeBak` | 308 | previous mode | 9 | 0 |
| 12 | `VideoModeChoisi` | 305 | chosen mode | 9 | 0 |
| 13 | `VideoModeVoulu` | 304 | wanted mode (0 -> forced 7 at boot, `sammypolish.wdl:411-412`) | 9 | 0 |
| 14-37 | `NbrNextraBall24` ... `NbrNextraBall1` | 259-282 | "extra balls earned" per level for the next level's lives (`Loads.wdl:15-73`) | all 0 | 0 |
| 38 | `NeverPlayed` | 72 | counter `+1` per game session start (`mm1.wdl:532`); `0` hides the Play menu on first run (`interfac.wdl:61`) | 4 | 0 |

[Q] `NbrNextraBallN` (numbered) are **never assigned anywhere** in the scripts (only the unnumbered `NbrNextraBall` is incremented, `mm1.wdl:1473-1522`), so they are always 0 and the bonus "next level extra life" feature never works. `EndTut/Think/JumpMessage` are only referenced by dead code. `Averagefps` etc. are machine state.

### 4.5 Load order and which file wins (`sammypolish.wdl:270-411`)
1. `Level1.SAV` .. `Level49.SAV` in order: after each load `LevelsInfo[N] = LevelsExist[N]` (**only element N of file N counts**, `:274-276`); `NbrLevelsWin` counts `== 2`; first non-zero `BonusState` becomes `BonusActif` (`:277-280`). A missing file leaves the previous array (zeros) -> `LevelsInfo[N] = 0` (level absent; e.g. 13-16, 29-32, 45-48 in this install). 
2. `Ver1.SAV`: `siger`, `RegistrationTypeOrg`, `UserName`, `NumberLast*Org` (copied to `NumberLast*` at `:293-295`, `Singer = Siger` at `:310`, name copy `:316-318`). Waits for `LevelEnCours == 999`.
3. `SearchNextLevel` (first `LevelsInfo != 0`) + `ValideLevel` rewrites that level's file (§4.1) (`:348-354`).
4. `BestSco0.SAV` last (`:394`): supplies `BestScores`, checkpoints, sliders, `JoyPad`, `ExtroVue`, `NeverPlayed`, `LevelEnCours` (resume level), video modes. Everything else in it (including its `LevelsExist` and `siger = 0`) is irrelevant/overridden (`Singer` keeps the registration, `LevelsInfo` was already copied).

Meaning of `LevelsInfo`: 0 absent, 1 present but must be won first ("Musisz zakończyć ...", `interfac.wdl:1030-1034`), 2 playable; 17 and 33 ship as 2 (first of each world, `MakeSAV :651-661`); 21 and 37 shipped 1 and are opened after registration (`InitLvl.wdl:482-489`).

### 4.6 The user's progress (decoded)
`level*.SAV` mtimes: shipped files 2002-07-11; user files `level2..9` (2025-09-23/24), `BestSco0` (2025-10-05), `level1` (rewritten 2026-10-11 by the boot `ValideLevel`).
- **Unlocked/playable**: levels **1-9** (`levelN.SAV` has `LevelsExist[N] = 2`), plus **17** and **33** (shipped 2). Everything else is locked: `level10/11/12`, `18-28`, `34-44`, `49` carry `LevelsExist[N] = 1`; bonus files 13-16/29-32/45-48 are missing.
- **Won**: levels 1-8 (each win wrote the next level's file; `level8.SAV` and `level9.SAV` share the minute 2025-09-24 18:27, `level10.SAV` was never rewritten). Level 9 was **attempted but not won**.
- **Best scores** (`BestSco0.SAV`): L1 540, L2 350, L3 90, L4 600, L5 960, L6 570, L7 750, L8 510, L9 390; all other 0.
- **Resume level** `LevelEnCours = 9`, `PreviousLevel = 9`, `NbrLevelsReach = 8`, `NeverPlayed = 4`.
- **Checkpoints**: none (`Player1RestartX/Y/Z` all 0). `Player1Start = (2790.50, -1073.70, 229.89)` is the stale default.
- **Options**: music 91.26, sound 100, `slider_video` 5 = mode 9 (1280×960), `ShiftPanelVideo` 415, `JoyPad` 0, depth 16, `siger 0` in this file (the shipped `Ver1.SAV` has `siger 1`, `NumberLast* = 12/44/28`, `BonusState 0`), `ExtroVue 0`.
- Earlier snapshots show the progression (level2.SAV bests 540/350 ... level9.SAV bests up to level 8, `Player1Start` of level2 = (-512, 268, 64.37), level9 = (1936, -1556, 576.37)).

---

## 5. Port recommendations

### 5.1 Save struct
```rust
#[derive(Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    pub level_state: [u8; 50],          // idx 1..=49; 0 absent, 1 locked, 2 unlocked/won
    pub best_scores: [i32; 50],
    pub checkpoints: [Option<[f32; 3]>; 50], // A4 coords (x fwd, y left, z up); None if x == 0
    pub last_level: u8,                 // LevelEnCours (resume)
    pub previous_level: u8,
    pub levels_reached: u8,
    pub games_started: u32,             // NeverPlayed
    pub extro_seen: bool,               // ExtroVue
    pub bonus_state: u8,                // BonusState/BonusActif
    pub last_levels: [u8; 3],           // tut/jump/think last open (12/28/44)
    pub options: Options,               // music 0..100, sound 0..100, video_mode, joypad
}
```
Registration (`siger`, `UserName`, `RegistrationTypeOrg`) is irrelevant for the remake: treat as registered (all 49 levels), keep an `unlocked_all` debug flag. `NbrNextraBall*`, `Averagefps`, `End*Message` can be dropped. Store as one RON/JSON file; write it after every game (as `BestSco0` is) and when a level becomes playable.

### 5.2 Importer outline for the original files
1. Read `levelN.SAV` for N in 1..=49 (skip missing -> `level_state[N] = 0`): `lzss::decompress(&bytes[8..], u32_be(bytes[4..8]))`, check header words `0x70C, 0x16, 0x03000134, 18`.
2. Parse 18 records (map ids per §4.4; unknown id -> error), then `u32` skill count and the 38 skills; ignore the tail. `value = i32 as f32 / 1024.0` (keep integer `/ 1024` for scores).
3. `level_state[N] = records[LevelsExist][N] as i32 / 1024` from file N only; treat `-1` as 1.
4. Parse `Ver1.SAV`: `last_levels = [Tut, Jump, Think]Org`; ignore `siger`.
5. Parse `BestSco0.SAV`: `best_scores`, checkpoints (`Some` if `RestartX != 0`), `last_level = LevelEnCours`, `previous_level`, `levels_reached`, `games_started = NeverPlayed`, `extro_seen`, options (`slider_video` 0..5 -> modes 2,5,6,7,8,9; for the port use windowed scale), `JoyPad`.
6. Ignore `Player1Start` (runtime), video depth/modes backups, string chunk.
For this user the import yields: unlocked 1-9, 17, 33; bests as in §4.6; resume level 9.

### 5.3 HUD implementation checklist (in order)
1. Render the HUD in a fixed 640×480 virtual canvas (mode 6 coordinates of §1.3), scaled uniformly; use nearest filtering and treat (0,0,0) as transparent (convert PCX to RGBA at load); `transparent` panels (gem plate, boost trails) at 50 % alpha [U].
2. Digit renderer: sprite sheet of 11 glyphs (`boink22/24`, `lucidd14`, `boink20`), zero-padded, low-`n`-digit clipping; lives = tens `lives/10` + ones `lives%10` on `fondlife` at (46,22)/(62,22); score at (519,7) 5 cells of 17 px; goal at (499,36) 5 cells of 15 px; ammo at (540,433) 5 cells of 16 px.
3. Static plates: `fondlife` (12,2), `fondscor` (464,4), `fondgems` (268,5, alpha 50 %), draw order by layer (1: plates, 2: bars/digits, 3-4: trails/lives text, 6: dino).
4. Hearts bar: 12×48 window at (0,2) over `vbarhear` rows `[hearts,hearts+48)` with `hearts = WindowHearts` 0..48 (4 hearts of 12 px, empties from the top).
5. Gem icon: 42×42 window at (298,29) over `hbargems` with offset `key*42` (0 none ... 7 rainbow); show with plate when a key is held; hide on deposit/eject; at goal reached force slot 7 and blink the icon every 12 frames (~4 ticks @16 Hz) once `score >= goal`, only while the options overlay is closed.
6. Boost bar: 150×8 window at (50,438) over `Hbarboos` cols `[b, b+150)`, `b = 150 - boost`; dino at (46,400) layer 6, trails at (6,400) layer 3 and (12,400) layer 4; visible only while boost > 0; drain 1 px/tick while Shift held; pickups add `Skill2` px.
7. Score goal flash: when touching an exit with score < goal show the goal in `boink20` for 4 ticks, normal for 4 ticks (cooldown 8).
8. HUD wasp (`panwasp.mdl`, skin = type 1..7, scale table §1.6) at camera-space (100,-45,-35) (screen ~ (570,434)) with `pan` 90 + 2°/tick; show only while ammo > 0, plus ammo digits.
9. Eye of Truth quad at camera-space (200,-45,-72) (screen ~ (445,440)), additive blend; hold 256 ticks then shrink Y/Z by 0.005/tick (196 ticks), reset on death.
10. Black fade quad (full-screen, skin 1 black / skin 2 brown gradient) at level start until `StartintroGame` finishes; reuse for menu transitions.
11. Popups: billboard sprites with frame strips (`plus50+6`: 6 frames of 30×12; `plu100+2`: 2 frames of 32×16; frame step 0.2/tick), motion toward camera at 20-40 quants/tick for 16-48 ticks; extra-life popup art must be made (originals missing).
12. Lose-a-life: flying heart (`lifeA.mdl`) with `particle_life` trail.
13. Fonts for menu text: 256-glyph CP1250 sheets (index = byte value, 32 cols); `lucida38`/`boink38` for title texts; convert the CP1250 WDL strings to UTF-8 and map back to CP1250 byte for the glyph lookup.
14. Win/lose: Lose logo (`Lose.mdl` quad, skin 256×81) flying in 20 -> 137.6 camera-space units in 8 ticks at 20 units above axis with amber light and smoke; stays until key or 320 ticks; Win: Sammy `Iwin` dance (12°/tick spin) until key; optionally the "Wygrałeś" quad (84.7×29.5 quants, depth 130, rainbow star particles).
15. Persist `BestSco`-equivalent after every game; update level states exactly as `ValideLevel`/`SearchNextLevel` (§3.5, §4.1).

### 5.4 Uncertain points to verify against the original under Wine
- Leading zeros and overflow rule of `DIGITS` (§2.3), blank vs `0` tens digit for lives < 10.
- Whether the Win logo is ever visible (§3.3).
- Exact blink rate in wall-clock terms (frame-based `ticks1`).
- Panel transparency/alpha for `transparent` flag and whether black is the colour key for `overlay`.
- Meaning of the 3 header constants and the string chunk in `.SAV` (§4.2): handled by pass-through, never needed.
