# Sammy Suricate (2002, Polish release): gameplay specification

This document specifies the game exactly as its WDL scripts implement it, as the reference for the Rust/Bevy port. The full title in the game is **"Sammy Suricate in Lion-Land" v1.42** (`SamTexts.wdl:143-193`). Engine details and porting advice are in `wdl-porting.md`.

**Conventions**
- Citations are `file.wdl:line` against `extracted/*.wdl`.
- **[R]** = read directly in the script.
- **[I]** = inferred.
- **[D]** = decoded from binary data (WMB entity records or `.SAV` files). The WMB entity layout used is: 8 little-endian f32 skills at action-name offset + 23. It is verified on config entities only; the position offset is unverified.
- Times are in **ticks (1/16 s)**. Distances are in **quants**.

**Heritage.** The code descends from the author's earlier ball/racket game "Ploing". Many identifiers are legacy:

| Legacy name | Meaning in Sammy |
|---|---|
| `BallesJoueur1` ("balls") | **lives** |
| `Blob` | the player |
| `Rakette` | racket (unused here) |
| `NbrAmmo1` | wasps carried |
| `TypeJeu` | game type; Sammy is `TypeJeu=25` in every shipped level [D] |

The Ploing-only branches (`TypeJeu` 1/8/16, extra-ball score thresholds, roulette and tile puzzles) are dead (`mm1.wdl:1740-1808`).

**Active compile defines** (`sammypolish.wdl:1-15`): `422`, `polish`, `frenchfullversion`, `tgawinlose`, `greenpepper`, `musique1`. Everything under `shareware`, `demoversion`, `demo`, `realnet`, `beta`, `softrender`, `voodoo`, `sammynewversion` is dead.

---

## 1. Game flow

### 1.1 Boot (`ACTION Main`, `sammypolish.wdl:226-597`) [R]
1. Disable Esc and show "Sprawdzanie systemu..." ("Checking system...") (`:227-229`).
2. Render setup: max 2000 particles and 1000 entities, `Floor_range=128` (`:231-240`). Try 800×600 at 16-bit, falling back to 640×480 (`:255-264`).
3. Run `Load_Info "Level",1..49` to fill `LevelsInfo[]` from each file's `LevelsExist[N]` (`:270-282`).
4. Randomise `MusicOrder[2..49]` to 1..8 (`:284-289`).
5. Run `Load_Info "Ver",1`, then wait up to 128 ticks for the sentinel `LevelEnCours==999`. `Load_Info` is asynchronous [I], and `MusicOrder` is re-rolled every frame during the wait, which acts as RNG seeding (`:291-307`).
6. Set `Singer=Siger` (registration flag), copy the username, then fix the songs:
   - levels 1-12: alternate song 1/2;
   - levels 17-20 and 33-36: random song 1..3 (`:308-345`).
7. Select the first unlocked level (`SearchNextLevel` + `ValideLevel`, `:348-354`).
8. `LOAD_LEVEL <menu_greenpepper.WMB>` (`:357`). If no level save exists, show "WARNING I FOUND NO LEVEL" for 32 ticks (`:385-392`).
9. `Load_Info "BestSco",0` restores best scores, sliders, video choice and checkpoint positions (`:394`). Apply the volumes and the user's video mode (`:397-419`).
10. Measure fps for 32 ticks. If `AverageFps<10`, show the English "game seems very slow" warning (`:545-588`).
11. `Branch IntroSammy` (`:592`). Numeric `ifdef 1..49` developer jumps go straight to a level instead; none are defined (`:421-543`).

### 1.2 Intro (`IntroSammy`, `Intro2.wdl:1-229`) [R]
The intro is not a video. It plays inside the menu level:
- **Logos** (`DisplayLogoA5`, `:924-1181`), billboarded in front of the camera:
  1. A5/3DGS: fade in about 25 ticks, hold 24, fade out 31.
  2. Publisher `publogo.mdl`: fade in, hold 32+24, fade out.
  3. Suricate `surilogo.mdl`: fade in, hold 64+24, fade out.
  - Logos 2 and 3 can be skipped with Esc or right-click.
- **Music:** `Play_Song Musique1` (sammy01.mid) (`:50`). Sets `PartieEnCours=1` and `PlayerInvincible=-1` (`:95-106`).
- **Walk:** a scripted Sammy (`InitPLayerPatrol`, `:231`) walks along `InitPathplayer` nodes (scan id 30). The camera follows with type-3 rules. A node whose `skill4==999` ends the intro (`:107-154`, `PathPlayerScanned :400`).
- **Skip:** Enter, Esc or right mouse ends the intro early.
- **Exit:** the camera glides to the menu origin (`MoveMenu`), then `Branch GestionJeu` → `MainMenuSammy` (`:159-229`).
- **Tutorial hints:** `InitTitle`/`Title_Hit` (`:492-606`) show proximity-triggered hint sprites. They are used in the intro and in tutorial levels (`Tut01E5b.pcx` …), at most 3 at once, and Bell1 plays in-game.

### 1.3 Main menu (`interfac.wdl:328-624`) [R]
- `MainMenu` sets `BallesJoueur1=0`, fades to black, reloads `menu_greenpepper.WMB` (retrying up to 16 times), sets the camera to `CameraPosOrigine` with arc 70, starts music 1 once, and forces at least 512×384.
- **The menu is a 3D scene.** Entities with `InitMainMenu` have `Skill1` = menu code. Mouse picking uses `Event_Touch/Click/Release` → `ObjetMenuSelect` → global `ChoixMenu`, which `MainMenuSammy` polls.
  - **Hover:** `Toc2` sound, an orange point light (255,180,50) with range 300, the icon scales ×1.2, and `particle_interface1` sparkles orbit it.
  - **Click:** `Ding1` sound.

| Skill1 | Item (title model) | Action |
|---|---|---|
| 0 | decor | n/a |
| 1 | Play (`titplay*.wmb`); hidden until `NeverPlayed>0` | `GestionPartie2` → `DebutPartie` at `LevelEnCours` |
| 2 / 3 / 4 | Tutorial (`tittutor`), Jump (`titjump`), Think (`titpuzzl`) | Hard **camera cut** to marker entities `posTutorialLevels/posJumpLevels/posThinkLevels` in the same scene, where the level icons are |
| 50 | Bonus (`titbonus`); only if `BonusActif==2`, or `==1 && ExtroVue` (`:40-55`) | Camera cut to `posBonusLevels` |
| 5 | Options (`titoptio`) | `InitGameOptions` overlay |
| 6 / 1001 | Quit (`titquit`) | Yes/No → `QuitSammy` |
| 20 | Back | Camera returns to the origin |
| 101+N | Level icon N (`icon1NN.pcx` / `ico100NN.pcx`) | `LevelEnCours=N` → `GestionPartie2` |

- **Icon states** come from `LevelsInfo[N]` (`interfac.wdl:96-190`):

| Value | State | Display / behaviour |
|---|---|---|
| 0 | absent | Ghosted inside the main ranges (1-12, 17-28, 33-44), otherwise invisible |
| 1 | locked | Transparent; click → `LevelNotReady` tooltip "Musisz zakończyć poprzedni poziom…" ("You must finish the previous level…"), plus beep2 |
| 2 | unlocked | Clickable |
| -1 | registered-only bonus | n/a |

- Hovering an icon shows two tooltip lines that follow the mouse: "Szkolenie N :" plus the title.
- With `Singer==1`, levels 21 and 37 are unlocked once level 20 or 36 respectively is unlocked (`:103-131`).
- The main menu shows `Texte_Version` "Sammy Suricate V1.42" and the username top-left.

### 1.4 Options overlay (`options.wdl`, `paninter.wdl`) [R]
- **Opening:** Esc (in-game or on the menu) or the Options item. Sets `Freeze_mode=1`, fog 70 (dimmed background), forces 640×480 while open, and shows the 640×480 panel `fondopt1.pcx`.
- **Tabs** (left column):

| Tab | Contents |
|---|---|
| **Audio** | Music slider 0-100 (`slider_musicvol` → `Midi_Vol`; plays `musique4` preview while dragging). SFX slider 0-100 (`slider_soundvol` → `Sound_Vol`; looping tiger roar preview). **Resolution slider** 1-5 → modes 512×384, 640, 800, 1024, 1280×960, with a label strip `hbarvide.pcx` shifted `V*83` px. **Keyboard / Joypad** toggle buttons (`JoyPad` 0/1). |
| **Help** | 7 pages of `Help1..7_Str` with arrow buttons; page 2 bitmap is blank (§13). |
| **Credits** | `lateam.pcx` plus `Thanksto_Str`; plays `Tigre_Son`. |
| Video tab, Control/Register tab | Video tab is not wired. The Control/Register button is a 4×4 blank bitmap and effectively dead. |

- **Navigation buttons:**
  - **Resume:** close the overlay.
  - **Back:** in-game, abandon the level → `MainMenu`; on the menu, close the overlay.
  - **Quit:** Yes/No dialog.
  - **Restart** (in-game only): Yes/No → `BallesJoueur1=0`, which ends the run.
- **Closing:** Esc or Enter (`Fin_MenuOptions`). Applies a video-mode change and restores key handlers.
- **No key remapping.** Option values are **only saved incidentally** inside `BestSco0.SAV` (§2), because every `Save_Info` dumps all saved vars.
- **Yes/No dialog** (`options.wdl:668-776`):
  - Panel `fondyn.pcx` at (223,135) with Yes/No buttons; the cursor is clamped inside the panel.
  - Keys J=yes and N=no always work. Language-specific Y/O do nothing because `language=1` (German) (`sammypolish.wdl:3`).
  - Port: accept T/Y/J for yes and N for no.

### 1.5 Starting a level (`DebutPartie`/`StartintroGame`, `mm1.wdl:542-1156`) [R]
1. `DebutPartie`:
   - Set `TypeJeu=25`, fade the camera to black, `Call InitLevelGlobal` (resets about 100 tuning globals, `InitLvl.wdl:1-194`).
   - Dispatch `InitLevelN` (`Loads.wdl`). Each does `load_level "<wmb>"; waitt Tempo_LoadLevel(4); Call AfterLoadInitLevel; BallesJoueur1 = NbrBallesStart + NbrNextraBallK`.
   - `AfterLoadInitLevel` (`InitLvl.wdl:195-238`) binds Space/Joy2 → `PlayerTire` and Ctrl/Joy1 → `PlayerJump`, sets camera arc 60 from `CameraPosOrigine`, and moves the player to the checkpoint if one was saved (`Player1RestartX[level]`).
   - The level's WMB config entities (`InitGame…`, `InitCamera…`) then override the defaults (§6.1).
2. Sun light 80, then `Branch StartintroGame`.
3. Wait for the mouse release, then retry until `InitCamera1` has set `CameraTypeStart` (up to 16 × 4 ticks).
4. **Level-start camera sweep** (type 3):
   - Start from `CameraPosOrigine`/`AngleOrigine` (from `InitGame2`), with `Camera3Friction = Camera3FrictionStart` (0.5) and `AdjustFpsCam3=0.1`.
   - Fog decreases 10/tick down to the level fog.
   - Ends when the camera converges (Δ ≤ 4), after 128 ticks, or on Enter/Esc/Joy2 (`:744-813`).
5. Then:
   - `Clip_Range = Clip_RangeGame*1.5` (`:815`);
   - `PartieEncours=1`, `PartieGagnee=0`;
   - lives clamped to `MaxBalles=19`;
   - `InitPanels` (HUD);
   - `PlayCurrentMusique` (`MusicOrder[level]`).
6. **Main loop** (`:863-909`) runs while `BallesJoueur1>0 && PartieGagnee==0`:
   - once `ScoreJoueur1>=ScoreGoal`, blink the gem panel every 12 ticks;
   - spin the HUD wasp;
   - update fps averages.

### 1.6 Win and lose (`mm1.wdl:910-1156`) [R]
- **The level is won (`PartieGagnee=1`)** when Sammy stands, grounded, on the exit tile (`InitExit`/`TileExit_Hit`, `mm1.wdl:3956-3973`: Win_Son, then 8 ticks).
  - The exit door has `Skill6==7` and opens only when `ScoreJoueur1>=ScoreGoal` (§4.3).
  - The same flag is also set by the extro end trigger and by a cheat.
- **The level is lost** when `BallesJoueur1` reaches 0 (§5.7).
- **After the loop:**
  - `Pov_Syn.IdPlayer=0`, so enemies ignore Sammy.
  - Clear the key bindings; `PartieEncours=0` stops all entity loops.
  - **Lose:** `Create <Lose.mdl>` at the camera running `InitTitleLoose` (`mm1.wdl:5314`):
    - flies in from depth 0→128;
    - amber light (250,200,50);
    - a lion-stand ping-pong animation emitting `particle_TileDeath` from random vertices.
  - **Win:**
    - `AnimGagnee` blends Sammy into the "Iwin" animation and spins him 12°/tick.
    - `Create <win.mdl>` running `InitTitleWin` (`:5203`) flies in, scale 0.32, with `Particle_Win` rainbow stars from random vertices.
  - Either screen lasts 320 ticks or until any key; the sun rotates 10°/tick.
- **Then:**
  - `StoreCurrentSkills` saves the checkpoint position.
  - If `ScoreJoueur1 > BestScore`, update `BestScores[level]`.
  - `Save_Info "BestSco",0` (with `Siger` zeroed while writing).
- **Win progression:**
  - `Facilite=0`, `ValideLevel` (mark level 2 and save `levelN.SAV`), `SearchNextLevel` (next existing slot), `ValideLevel`; `NbrLevelsReach++`.
  - After the last level of a world (`NumberLastTut/Jump/ThinkLevel` = **12/28/44** from the shipped `Ver1.SAV` [D]; source defaults 12/20/36), branch to `MainMenu`.
  - If both 28 and 44 are completed, branch to **`InitExtro`** (`:1102-1106`).
  - Otherwise `Branch DebutPartie`, which starts the next level directly.
- **Lose progression:** `Facilite++` (no live consumer in Sammy) and the same level restarts.

### 1.7 Extro (`Extro.wdl`) [R]
A **playable credits level**, `extro.wmb`, not a cutscene.
1. Set `ExtroVue=1` (saved; unlocks the Bonus menu item) and `LevelEncours=49`, then load.
2. Run the same camera sweep as a normal level start, with `BallesJoueur1=1`, `MenuOptionsGame=2`, and `LevelEncours=20`, so quitting doesn't replay the extro.
3. Sammy uses a simplified controller (`SammyExtro`, `:274`): no jump or fire, but Shift boost.
4. The level contains:
   - credit sprites (`InitTitle`: agameby/jerome/jorg/remi/theend.pcx);
   - camera retarget triggers (`InitSetPovSyn`, `InitSetAnimPlayer`);
   - "You Win" wasps (`InitWaspEnnemy` → `InitWaspExtro`, IdEnnemy 99 emitting `Particle_Win`);
   - lions and keys.
5. `InitSetFinExtro` (`mm1.wdl:5531-5562`): when Sammy enters its range, fog colour 1 goes to fog 100 at 6/tick, a black panel appears, and `PartieGagnee=1` → `MainMenu`.
6. No music call: the previous level's music continues [I].

### 1.8 Quit
`QuitSammy` (`interfac.wdl:654-830`) saves `BestSco0`. Because `frenchfullversion` is defined, it exits after 4 ticks with no end panel. F10 calls `PreQuitSammy`, which quits only if X is held.

---

## 2. Saves (`.SAV`) [R calls, D format]
- **Container:** `u32 BE compressed_len`, `u32 BE uncompressed_len` (always 2656), then Okumura LZSS (the same variant as the WRS archive: 4096 ring buffer prefilled with 0x20).
- **Payload:** 18 `Var_Info` records `{u32 id, u32 count, i32 vals[count]}`, then 38 named `Type Global` skills `{cstring, i32 x,y,z}`. Values are 22.10 fixed point (`/1024`).
- **Every `Save_Info` writes the whole set, and every `Load_Info` overwrites the whole set.** The meaning of a file therefore depends on which slot is relevant:

| File | Written by | Meaningful content |
|---|---|---|
| `levelN.SAV` (N=1..49) | `ValideLevel` (`InitLvl.wdl:256`) on win | `LevelsExist[N]` (1 locked, 2 unlocked) |
| `Ver1.SAV` | registration (`InitLvl.wdl:379,466`) and dev `MakeSAV` (`sammypolish.wdl:705`) | `siger` (1 = registered; shipped = 1), `NumberLast*LevelOrg` 12/44/28, `LevelEnCours=999` sentinel |
| `BestSco0.SAV` | end of every game (`mm1.wdl:981`), quit (`interfac.wdl:681`), registration | `BestScores[50]`, `Player1RestartX/Y/Z[50]` (checkpoints), sliders, `JoyPad`, video mode, `NbrNextraBall1..24`, `NeverPlayed`, `ExtroVue`, `BonusState`, `LevelEnCours` |
| `Ver0.sav` | n/a (not a save) | CP1250 text for the Windows launcher: renderer choice `sammypolish`, `-crd 2`, `-w3d` |

**Load order matters:** Level1..49, then Ver1, then BestSco0. Missing `level13-16/29-32/45-48.SAV` keep the bonus icons hidden.

**Port recommendation:** a single serde struct `{levels: [u8;50], best: [i32;50], checkpoint: [Vec3;50], extro_seen, bonus_state, options{music,sfx,video,joypad}, last_level, never_played}`. Optionally write an importer for the original `.SAV` files (the user's current progress is levels 1-9 [D]).

---

## 3. Levels

### 3.1 Level table (`Loads.wdl`; numbers decoded from each WMB's `InitGame`/`InitGame2` [D])
Categories:
- **1-12:** Tutorial, "Szkolenie N" (Training).
- **17-28:** Jump, "Ćwicz zręczność N" (Practice dexterity).
- **33-44:** Think/puzzle, "Ćwicz myślenie N" (Practice thinking).
- **13-16, 29-32, 45-48:** Bonus 1-12.
- **49:** EXTRO.

Titles are in `panels.wdl:81-190`, encoded CP1250.

Music:
- Levels 1-12: odd = `sammy01.mid`, even = `sammy02.mid`.
- Levels 17-20 and 33-36: random from songs 1-3, fixed per boot.
- Other levels: random from songs 1-8 per boot (song 8 = sammy01). See §10.

| Lvl | WMB | Goal (score) | Start lives | Notes |
|---|---|---|---|---|
| 1 | 10001 | 540 | 1 | starts unlocked |
| 2 | 10002 | 350 | 1 | |
| 3 | 10003 | 90 | 1 | |
| 4 | 10004 | 600 | 5 | first lions |
| 5 | 10005 | 960 | 5 | first wasps |
| 6 | 10006 | 570 | 5 | platforms |
| 7 | 10007 | 750 | 5 | boost |
| 8 | 10008 | 510 | 5 | teleports |
| 9 | 10009 | 420 | 5 | restart point |
| 10 | **10012** | 1170 | 4 (hard-coded `Loads.wdl:80`) | file order swapped |
| 11 | **10010** | 180 | 3 | weak doors |
| 12 | **10011** | 450 | 1 | hearts; end of tutorial world |
| 13-16 | `bonus1..4.WMB` | n/a | n/a | **files missing**: bonus levels are cut |
| 17 | 101o_j | 1440 | 5 | starts unlocked |
| 18 | 104m_j | 1500 | 3 | |
| 19 | 103j_j | 990 | 2 | |
| 20 | 112e_j | 900 | 5 | |
| 21 | 107i_j | 600 | 3 | |
| 22 | 174a_j07 | 1000 | 3 | "red-gem fall" |
| 23 | 111r_j | 150 | 5 | |
| 24 | 172c_j06 | 720 | 3 | "nano lions call their friends" |
| 25 | 175c_t | 990 | 3 | `_t` file in the jump world |
| 26 | 177e_j | 950 | 3 | hives |
| 27 | 108i_j | 300 | 3 | "dark cave", Eye of Truth |
| 28 | 179f_t | 4300 | 2 | "slides in Sammy's land" (ice); end of jump world |
| 29-32 | `bonus5..8` | n/a | n/a | missing |
| 33 | 106g_t | 510 | 3 | starts unlocked |
| 34 | 171c_t04 | 450 | 2 | |
| 35 | 110n_t | 660 | 1 | "teleport me v1" |
| 36 | 180c_t | 450 | 2 | "mosaic" |
| 37 | 170b_t00 | 870 | 3 | |
| 38 | 102n_t | 900 | 1 | |
| 39 | 173b_t05 | 540 | 2 | "the trees" |
| 40 | 114t_t | 150 | 1 | |
| 41 | 178d_t | 150 | 2 | "big wooden cage" |
| 42 | 176g_t | 2220 | 1 | "great pyramids", hive |
| 43 | 113q_t | 300 | 1 | |
| 44 | 109o_t | 330 | 1 | "teleport me v2 (hard)"; end of think world; `SousTitre44` "Hint : Yellow is the key" |
| 45-48 | `bonus9..12` | n/a | n/a | missing |
| 49 (50) | extro | 2220 | 1 | credits level |

**Referenced vs unused files:**
- All 36 real WMBs in `levels/` are referenced.
- The 13 `b113…b148.wmb` files in `levels/` are identical 4756-byte stubs with no entities and are never loaded. The `b113-b116.wmb` in the WRS root are bonus-icon models used by the menu.
- `menu1.WMB` (the non-greenpepper menu) is absent.
- `InitLevel50` duplicates 49.

### 3.2 What a level contains (counts from `strings` [D]; full table in the scratch notes)
- **Every level has one of each config entity:** `InitMarche` (player, `suricat1.mdl`), `InitDecor`, `InitSkyMenu`, `InitGame/2/3`, usually `InitGame4/5`, sometimes `InitGame6/7`, `InitCamera1/2/3` (sometimes 4), `InitExtraBalles`, `InitNExtraBalles`, `Initpublic`, and one or two `InitExit` (`tileexit.wmb`).
- **Gameplay entities**, with their typical model/prefab:

| Action | Prefab / model |
|---|---|
| `InitBonus3` | gemgr+17.pcx |
| `InitKey` | gemre/ye/or/bl/ma/ve/mu+17.pcx |
| `InitDoor` | door*/dfix*/done*/doorexit.wmb |
| `InitDoorWeak` | doorwk1-4.wmb |
| `InitLion` | nanolion/rasta/hippie/rockera/punk/scorpion.mdl |
| `InitAmmo1` | wasp1.mdl |
| `InitHive` | hiveblue/hivegree.wmb |
| `InitPlatform` | platfor1-3/platf*/wagongem.wmb |
| `InitEnginePlatform` | invisible |
| `InitTeleport` | teleport/telelion.wmb |
| `InitTiledeath` | tiledeat.wmb |
| `InitRestart` | restart.wmb |
| `InitBoost` | boost.mdl |
| `InitSmallHeart` | heart+11.pcx |
| `InitBonusVie` | lifea.mdl or coeur.wmb |
| `InitEyeOfTruth` | eyetruth.pcx |
| `InitBoulder` | boulder3-5.wmb |
| `InitPath` | setpath.wmb, spheran*.mdl |
| `SetCamera` | setcam.wmb, cliprang.mdl |
| `InitStartEnnemy` | cliprang.mdl |
| `InitArbre` | herbes1/tree1b/tree2.pcx |
| `InitMDL`/`initmdl` | cactus1, huttop2, grate1.mdl |
| `OnGround` | arbre1.pcx |
| `InitTitle` | tutorial hints |

- `Bind`-listed prefabs not seen in any level entity: `barrier0-3.wmb` and `hivemult/hivestri.wmb`. They are probably level geometry or unused [I].
- **Elephant** (`IdEnnemy` type 1) has code but no model in any level [D]. **NPCs** hippie/punk/rasta/rocker are lion variants (§8), not friendly NPCs.

---

## 4. Core rules

### 4.1 Score, goal, collectibles
- **Diamonds** (`InitBonus3`, `mm1.wdl:1200-1291`, sprite `gemgr+17.pcx` frames 1-17 at 3 frames/tick):
  - Picked up by the player's pickup scan (scan id 1, radius `BlobZoneScan` = player Skill5 = **104** in most levels [D]).
  - Each gives `Skill2` points (30 typical; 50/100/150/300 rare [D]) via `AddPoints`.
  - Pickup effects: Diam_son, 15 `particle_ExploseDiam`, a shrink ×0.8/tick, and popups `plus50+6.pcx` / `plu100+2.pcx` for 50/100-point diamonds.
- **`AddPoints`** (`mm1.wdl:1440-1610`): `ScoreJoueur1 += Points`, clamped at ≥0. On first reaching `ScoreGoal` (`InitGame.Skill7`):
  - plays `ScoreReach_Son` (applause);
  - sets `PlayerKey=255` (master key) and `PlayerOldKey=999` (no more key-gem pickups);
  - shows HUD gem slot 7;
  - **all key gems vanish**, because their idle loop ends when `score>=goal` (`gems.wdl:77`).
- Extra-life score thresholds (`ExtraBalle1..8`) are all 0 in shipped levels [D], so score never grants lives.

### 4.2 Colour keys (key gems, `gems.wdl`)
- **Colours** are a bitmask in `Skill6`:

| Value | Colour | Sprite |
|---|---|---|
| 1 | red | gemre |
| 2 | yellow | gemye |
| 3 | orange (1+2) | gemor |
| 4 | blue | gembl |
| 5 | magenta (1+4) | gemma |
| 6 | green (2+4) | gemve |
| 255 | multi (opens anything) | gemmu |
| 7 | exit door / "all" | n/a |

- Sammy carries **one key** (`playerKey`). Picking up a second gem:
  - **Mixable pair** (1+2, 1+4, 2+4 in either order): becomes the mixed colour, with the mix animation and GemMixing sound.
  - **Otherwise:** the old key is **thrown back out** as a bouncing gem (`EjectOldKey`/`LaunchYouKey`) that becomes collectable again after its 5th bounce.
- Pickup is blocked while a previous key is in flight (`PlayerOldKey != 0`).
- Key gems give **no score** (the `BonusGem` call is commented out, `gems.wdl:344`).
- **Gem editor parameters** (`gems.wdl:1-37`):

| Field | Meaning |
|---|---|
| Skill6 | colour |
| Skill5 | "surprise": random colour up to +Skill5, capped at 6 |
| Skill9 | lifetime in ticks (blinks, then explodes; blink sound capped at 5 at a time) |
| Skill3 | invisibility ticks after being crushed |
| Skill4 | spin speed |
| Flag5 | keep its height above ground |
| Flag6 | keep its WED height (no ground snap) |
| Flag7 | gravity / carried by platforms |

- **Death reset:** on player death, scan id 1000 respawns all level-placed gems (Flag3) and removes runtime-spawned ones. Exception: if `KeyPlayer1==255`, all are removed.

### 4.3 Doors (`mm1.wdl:2852-3283`; prefabs `door<colour>`, `dfix<colour>`, `done<colour>`, `doorexit`, `dfixall`)
- **Typical WED values** [D]: Skill2=250 (raised to at least 300: the rise height), Skill4=40 (up speed per tick), Skill5=0 or 80 (down speed), Skill6=colour, Skill7=25 (reacts to the player's per-tick door scan, id 25, radius 500), Skill3 defaults to 350 (open distance).
- **Open rule** (`movePortail :3015`) for Sammy (`Flag2==0`):
  - `PlayerKey==255` → open.
  - `Flag6` → open without a key.
  - `Flag7` (exact): `key == Skill6`.
  - Otherwise: `(key & Skill6)!=0` and (`key==Skill6` or key is a primary 1/2/4).
  - Wrong key or no key: the door flashes ambient for 3 ticks.
  - **Exit door** (`Skill6==7`) while the score is below the goal: also blinks the score panel (`WarnScoreGoal`).
- **Other openers:** Flag3 → lions (8888/1111); Flag4 → wasps (9999); Flag5/6 → legacy balls.
- **Open sequence** (`OuvrePortail :3114`):
  1. Distance must be `< Skill3-4`.
  2. DoorOpen_son plays; the door rises to `Z0+Skill2` at `Skill4`/tick.
  3. It holds `Skill8` ticks (default 4), refreshed while Sammy stays in range. If `Skill8>48`, the last 48 ticks blink with Bell1 every 12 ticks, then Bell2.
  4. Before lowering, it rescans (id 102, `DoorScanEntities`) until no Sammy or lion is underneath. Then DoorClose_son plays and it lowers at `Skill5`/tick.
- **Door variants:**
  - **`Skill5==0`:** "one-shot" door that stays open (ambient -20).
  - **`Flag5` (permanent door):** the held gem **flies from Sammy into the door** (`CreateGemKeyAnim`/`AnimGemKey`, `gems.wdl:987-1198`, speed `SpeedGemKey=12`, max 80+16 ticks):
    - **If Sammy walks away early,** the gem returns to him.
    - **On arrival,** the gem explodes, the door is validated (`Skill15=1`), its model morphs to `doordark.wmb`, and it stays open. The player loses the key.

    The in-game help describes this as "door 3 catches the gem".
  - Prefab families [I from names]: `dfix*` = permanent (Flag5), `done*` = one-shot, `door*` = timed.
- **Weak doors** (`InitDoorWeak`/`DoorWeak_Hit`/`ExplodeDoorWeak`, `mm1.wdl:4466-4707`):
  - Hit points = Skill1 (or Skill2), reduced by wasp poison. Each hit morphs through `doorwk4→3→2` with Plank1_son.
  - At 0, the door turns into 6 bouncing planks (`plank1-3.mdl`, `Launchplank`).
  - Lions break them when their weight is ≤0.4, or with an attack swipe.

### 4.4 Lives and health
- **Lives:** `BallesJoueur1`, at most 19.
  - Start value: `NbrBallesStart` (InitGame2.Skill1) plus carried `NbrNextraBallK`.
  - **Big heart** `InitBonusVie` (`mm1.wdl:2633-2715`, `lifea.mdl`/`coeur.wmb`): +Skill2 lives (usually 1), green light (50,250,10) with range Skill3, `GetLife_Son`, emits `particle_getlife`. Ignored when lives are at the maximum.
- **Health:** `WindowHearts` 0..48 (`WindowHeartsOrg=48`).
  - **Small heart** (`InitSmallHeart`, `mm1.wdl:4721-4815`, `heart+11.pcx`, 11 frames): +Skill2 (default 12), capped at 48, with Gagne1 and Chimes2 sounds and 30 `particle_SmallHeart`.
  - **If health is already full,** the heart shrinks, hides for 48 ticks and returns.
- **Damage sources** (`WindowHearts`):

| Source | Damage | Where |
|---|---|---|
| Lion swipe (`LionEtat 10`, `AnimStep>60`, once per swing) | `EnnemyResistance × HargneEnnemy(2)` | `player.wdl:560-572`, `lion.wdl:495` |
| Lion running into Sammy (state 2 → starts swipe) | `EnnemyResistance × 0.25` | `player.wdl:573-583` |
| Lava (grounded) | 1 per tick | `mm1.wdl:4886-4901` |

  - At ≤0 → `Player_Kaputt`.
- **Instant death tile:** `InitTileDeath`/`TileDeath_Hit` (`mm1.wdl:4052-4062`) runs `My=You; Branch TeleportFor1Life` when Sammy stands grounded on it. `TeleportFor1Life` (`player.wdl:699-759`, its only caller) works like this:
  1. Ends if already dead.
  2. Disables input (`PlayerMoveMode=0`), becomes passable, sets `IdPlayer=0`, and zeroes all forces.
  3. Calls `ResetSammy`, which ejects the held key.
  4. Spawns a flying heart.
  5. `BallesJoueur1 -= 1`. **If no lives remain, it simply ends**, and the main loop shows the lose screen.
  6. Otherwise it fades alpha -2/tick (about 50 ticks), moves to `Player1Start`, places the camera proxies at +100 z, and branches to `RestartSammy`.

  Hearts are not involved, and there is no death animation. This matches `Help6_Str`: "a deadly tile costs a life and returns Sammy to the start".
- **No fall damage**, and no kill-plane in the scripts.

### 4.5 Boost and wasp ammo
- **Boost** (`WindowBoost`): `0` = full, `150` (=`widthbarBoost`) = empty. It starts empty.
  - Pickup `InitBoost` (`boost.mdl`, `mm1.wdl:4315-4464`): `WindowBoost -= Skill2` (50 typical [D]), floored at 0, with chimes2 and a light flash. If already full, it hides for 64 ticks.
  - **Holding Shift (or Joy3) while charged:** costs `ConsoBoost=1` per tick and roughly doubles forward speed (§5.5).
- **Wasps:** see §9. HUD ammo is `NbrAmmo1`.

### 4.6 Other level objects (all [R] in `mm1.wdl` unless noted)

| Action | Line | Behaviour | Editor params |
|---|---|---|---|
| `InitExit` / `TileExit_Hit` | 3956 | Win when Sammy is grounded on it (sonar, `z <= ground+BlobHaut+41`, vspeed 0) | n/a |
| `InitTeleport` / `Teleport_Hit` | 4099-4300 | **Sammy:** fades out (alpha -10/tick with particles), moves to (Skill1,2,3) facing Skill4, the camera snaps behind him, then he fades back in. Locked while busy. **Lions:** also teleported if Flag2. Idle `particle_Teleport` every 4 ticks unless Flag3. | Skill1-3 destination, Skill4 pan, Flag2 lions too, Flag3 no particles |
| `InitRestart` / `RestartHit` | 3771-3861 | Checkpoint (green arrow, `restart.wmb`): hovers and spins. On pickup scan: Restart_son and particle bursts; `Player1Start = pos+(0,0,150)`, `player1AngleStart.pan=Skill8`; camera restart values = current; hides itself at z=-9000. Respawns on the reset scan (id 1000). | Skill2-4 spin, Skill7 int=hover amplitude / frac=damping, Skill8 restart pan |
| `InitTileDeath` | 4023 | Deadly tile for Sammy (→ `TeleportFor1Life`: lose a life, respawn). Lions with Flag2 die. Emits `particle_TileDeath` every 2 ticks unless Flag3. | Flag2 lions die, Flag3 no particles |
| `InitDeathDalle` | 3574 | Deadly tile for non-player entities listed in Skill2-4 (sets `LionEtat=Skill5`, e.g. 1001 vanish, 1002 explode) | n/a |
| `InitLava` / `Lava_Hit` | 4869-4938 | Hearts -1/tick for Sammy. Enemies lose resistance and flee (`EnnemyFuit`); scorpions are slower to burn. | Flag3 no particles |
| `InitTrampoline` | 3506-3572 | Bounces entities whose id is in Skill2/3 (or any, with Flag1): `vspeed = Skill5×int(VSpeedOrg)`, friction ×Skill6. Flag2: slow/accel tile (Skill4). Flag3: rotate. Can link to a platform engine. | Skill1 link id, Skill2-8 |
| `InitPlatForm` / `InitBodyPlatForm` / `InitEnginePlatForm` | 2238-2629 | **Moving platforms.** An invisible *engine* moves along its local +X at `Skill2`/tick and links all bodies with the same Skill1 (default 8465) via scan. On impact it either reflects (Flag3) or pauses `Skill3` ticks and turns by Skill4/5/7. **Locked platforms** (engine Skill6 = key colour, same rule as doors, Flag7 exact) start only when Sammy stands on them holding the key, which morphs the platform to `platfor1.wmb` and plays platform1_son. Riding Sammy is carried (`PlayerForce`). Shift-boost ×3 speed while riding. | Engine Skill1-7, Flag3/7; body Skill1-2 |
| `InitBoulder` / `BoulderGem_Hit` / `EmitGems` | `gems.wdl:1568-1657` | Rock that emits key gems periodically (Skill8 ticks) or when hit by entity id Skill5. Skill1 = gem fate: 0 decorative, <1000 lifetime, 1000 stays, 1001 persistent. Skill6 colour (999 random). | Skill1-12, Flag2-6 |
| `InitEyeOfTruth` / `EyeOfTruth_Hit` | 4954-5103 | Power-up showing enemy detection rings (`DisplayEnnemyRAnge`) for `DureeEyeTruthOrg=256` ticks, then the HUD eye shrinks away. Respawns after 128 ticks. | Skill2-4 spin, Skill7 hover, Flag3/4/6 |
| `InitBonus5` / `InitMdlBonus` | 2046-2214 | Trigger that plays a linked model's ping-pong animation with a light and popup. No score in Sammy mode. | n/a |
| `InitRotationObjet` / `InitRemoveObjet` | 3619-3694 | Decorative spinner/hover; object removed on scan id = Skill2 | n/a |
| `InitStartEnnemy` | 5132 | Invisible zone: sets enemies with `IdEnnemy==Skill1` to state Skill3 and adjusts their ranges (Skill5/6), re-arming every Skill4 ticks | n/a |
| `InitSetPovSyn` / `InitSetAnimPlayer` / `InitSetFinExtro` | 5404-5562 | Extro triggers: camera target switch, forced Sammy anim, end of game | n/a |
| `SetCamera` | `camera.wdl:958` | Camera zones (§6.4) | n/a |
| `InitPath` | `lion.wdl:774` | Patrol node for lions: Skill1 path id, Skill2 next id, pan/tilt = heading | n/a |
| `InitArbre`, `InitMDL`, `OnGround`, `Initplanche1`, `InitDecor`, `InitSkyMenu`, `Move_Sky`, `InitDynamicLight` | 1652-1733, 3285-3492, 3911, 4817 | Decoration: ground-snapped trees/sprites with random skin/frame/scale. Thin trunk hull ×0.3 when Flag6==0. Sky models follow the camera and rotate (`InitSkyMenu` tests Skill5/6/7 but applies 2/3/4: bug, replicate). Decor at z=Skill3 (default -500). | per header comments |
| `InitGame…`, `InitCamera1-4`, `InitLionSkills1/2`, `InitExtraBalles`, `InitNExtraBalles` | 1740-1985, 2719-2851 | Per-level tuning entities (§6.1) | n/a |
| `Initpublic`, `Initprotection` | 1981-2025 | DRM: **drop** | n/a |

---

## 5. Player (Sammy)

### 5.1 Controls (`InitLvl.wdl:219-223`, `mm1.wdl:108-128`, `player.wdl`) [R]

| Input | Action |
|---|---|
| Arrows (`Key_Force`) / stick (`Joy_Force*0.5`) | Left/right turn, up/down forward/back ("tank" mode, `PlayerMoveMode=1`). Camera-relative mode 2 exists but is never selected in shipped levels. |
| Ctrl / Joy1 | Jump (`PlayerJump`) |
| Space / Joy2 | Throw a wasp (`PlayerTire`) |
| Alt / Joy4 | Quick 180° turn (`RetourneSammy`) |
| Shift / Joy3 | Boost (needs charge) |
| Esc | Options overlay |
| F5 / Shift+F5 | Cycle resolution |
| F10 + X | Quit |
| M / F8 | Toggle music 1↔2 / mute |
| Enter / Esc / Joy2 | Skip the level-start camera sweep |

**Debug and cheat chords**, all bound in the shipped game:
- R: restart. R+Shift+X: instant win.
- S+Alt: pause / frame-step.
- E+Shift+X: go to the extro.
- C: cycle camera type.
- A+Shift+X: ammo 100.
- B+Shift/O: full boost.
- U+I+V: invincible.
- L+V+Shift: all levels.
- K+Y+Shift: master key.
- T+Shift+O: spawn a key.
- Tab+Ctrl+I: `EXECUTE` console.

Port these behind a debug flag. The mouse is not used in gameplay.

### 5.2 Body and constants
Values come from `InitLevelGlobal` (`InitLvl.wdl`) unless noted. Per-level `InitGame3/5/6/7` may override them.

| Name | Value | Source |
|---|---|---|
| Model scale | 8 (`suricat1.mdl`) | `player.wdl:34-36` |
| Centre height above floor (`BlobHaut`) | 4.4×8 = **35.2** | `mm1.wdl:36`, `player.wdl:71` |
| Step-up probe (`playerClimb`) | 8 (sonar cast from z+8, so floors up to about 43 above the current floor snap) | `mm1.wdl:101`, `player.wdl:1211` |
| Collision | `fat=1, narrow=0` (hull from model bbox) [I] | `player.wdl:55` |
| Walk speed (`SpeedBalle` = WED Skill2) | **25** quants/tick (23-30 in some levels) [D] | `player.wdl:1156` |
| Pickup scan radius (`BlobZoneScan` = WED Skill5) | **104** (64-100 in some levels) [D] | `player.wdl:217-226` |
| Door/camera-zone scan radius | 500, id 25, every frame | `player.wdl:233-238` |
| Turn rate standing / moving | 10 / 7 °/tick (×0.6 when `Intention.y>0.55`, i.e. boosting; ×0.5 in air) | `player.wdl:1125-1148` |
| Gravity start / accel (walk-off) | -2 / 1.5 per tick² | `player.wdl:1255-1259` |
| Jump impulse / decel | 30 / 3 per tick² (fall after a jump also uses 3) | `InitLvl.wdl:60-61`, `player.wdl:428` |
| Air control | `vx = 1 + Intention.y×25×0.75` | `player.wdl:1193` |
| Fall animation trigger | drop > BlobHaut+64 (ground) or +128 (air) | `player.wdl:1260,1309` |
| Hard landing | impact vspeed < -70 → slower recovery | `player.wdl:481` |
| Ceiling probe | 32 above vertex 16; cancels ascent | `player.wdl:1291-1307` |
| Weight / own push | 0.8 / 1.8 | `player.wdl:121-124` |
| Knockback | `IntensiteBlobBounce=30`, decay `AmortiBouncePlayer=10 × 80/(fps+1)` per tick | `player.wdl:545-554, 1415-1470` |
| Platform force decay | ×0.8 per tick (`accel …,0.2`) | `player.wdl:1232-1245` |
| Local velocity damping | `Accel VitesseRaq,0,0.8` each frame | `player.wdl:248` |

### 5.3 Movement model (`HandleSpeedPlayer`, `player.wdl:1115-1488`) [R]
Order of steps each tick:
1. **Turning:** `pan -= Intention.x × rate × time`.
2. **Forward:** `vx = Intention.y × time × 25`.
3. **Ground probe:** sonar down 2000 → `BlobHautSol`.
4. **External platform force:** move by `PlayerForce` with entity events disabled, then decay.
5. **Slope handling** (`handlePente :1490-1639`):
   - **Condition:** `normal.z ≤ 0.92` (about 23°) and close to the ground.
   - **Slide:** the slide velocity accelerates `1.5×(1-normal.z)` per tick toward `normal.xy×100`, with z=-20. The model leans up to 12° tilt / 6° roll, and dust is emitted.
   - **Big slope** (`|pente|>10`): forward ×0.75, steering becomes strafing, the body turns downhill at 0.05×Δ, and the looped `Glisse_Son` plays with pitch tied to slope.
   - **Medium slope** (`|pente|>2`): forward ×0.9, strafe ×0.5, turning ×0.25.
   - **Jump restriction:** jumping on a slope is only allowed when facing within 45° of downhill.
6. **Vertical motion:**
   - When grounded: snap to `ground+35.2`. If the floor is more than 1 below, start falling.
   - In the air: `vz -= fric×time`, landing clamps to the floor.
7. **Move:** `Move My, VitesseRaq, Abspeed`. If moved ≤0.2, record speed as blocked.
8. **Knockback:** apply the knockback vector separately, ignoring entities.

### 5.4 Jump, fire, U-turn
- **Jump** (`PlayerJump :385`, `FinJump :435`):
  - Requires `PlayerEtat==1` and grounded.
  - **Standing:** plays the wind-up "sautA" at 30%/tick first, then jumps.
  - **Moving:** jumps immediately.
  - `vz=30`, decel 3. `FinJump` waits until grounded (with `LoopSecurity` 64 ticks), plays "sautC", then returns to state 1.
  - **Stomping a lion** (only if `BounceOnEnnemy != 0`, default 0 = disabled): bounces Sammy with Blouit_Son.
- **Fire** (`PlayerTire :271-384`):
  - Needs `NbrAmmo1>0`, otherwise plays error1.
  - Animation "TirA" then "TirB" at 20%/tick × per-type factor {1, 2.2, 1.5, 1.2, 1.2, 1, 3}.
  - Spawns `wasp1.mdl` 40 quants **behind** (pan+180) and 50 up. Verify which way the model faces.
  - **Types 3 and 4:** fire a 3-wasp fan (pan -5/0/+5, 2 ticks apart, 1 ammo each).
  - At 0 ammo the HUD wasp hides.
- **U-turn** (`RetourneSammy :1642`): Blouit_Son, "Turn" anim at 28. Pan approaches +180 with step `Δ×0.25`, then `Δ×0.4` per tick, stopping when the step is ≤2 (lands about 5° short, no snap).

### 5.5 Boost (`HandleBoostPlayer :1763`, `LessBoost :1799`)
- **Keyboard:** the engine doubles `Key_Force` while Shift is held [I].
  - **With charge:** turn ×0.75, forward stays ×2, and `WindowBoost += 1/tick` (about 150 ticks per full bar); `particle_SpeedySammy` every 2 ticks.
  - **Without charge:** ×0.5, giving normal speed.
- **Joypad:** with charge, forward ×2.

### 5.6 Animations (`anims.wdl:352-497`, `mm1.wdl:39-85`) [R]
Animation is driven by `PlayerAnimStep` 0-100 (% of the scene), advanced by speed×time.

| State | Scene | Mode | Speed | Next |
|---|---|---|---|---|
| 1 Stand | stand | cycle | 2 | n/a |
| 2 Walk | walkB | cycle | 0.5 × forward speed (plays backward when reversing); dust every 4 ticks | n/a |
| 3 Turn | Turn | cycle | 14; dust every 8 | n/a |
| 4 Walk→Stand | raw frames +0.07/tick | n/a | n/a | Stand |
| 5 / 6 Fire | TirA / TirB | frame | 20 | FinTir → Stand |
| 7 / 8 / 9 Jump | sautA / sautB / sautC | frame | 30 / 14 (30 near landing) / 14 (18/36 if input, ×0.5 hard landing) | n/a |
| 10 Death | kaputt | cycle | 3 | game over: loop "finkaputt" |
| 11 / 12 | loopjambes / salut | cycle | 2 / 3 | salut → loopjambes |
| Win | Iwin | blended in by `AnimGagnee` | n/a | n/a |

### 5.7 Death and respawn (`Player_Kaputt :772-881`, `RestartSammy :884-993`)
Skipped if `PlayerInvincible ≤ 0`.
1. `WindowHearts=0`; hide the Eye of Truth.
2. Flying heart `lifeA.mdl` (`Animmoins1life`) with Heart_away.
3. **Drop the key:** if holding a key and the score is below the goal, the key is ejected as a gem (`EjectNewKey`, vz 40).
4. Death anim with 12 orbiting stars (`EtoilB+9.pcx`); the camera tilt drops; Kaputt_son at tick 8.
5. **Lose a life:** `BallesJoueur1 -= 1`.
6. **If no lives remain:** the camera orbits (+3°/tick) and the "finkaputt" loop plays.
7. **Otherwise:**
   - blink for about 46 ticks;
   - **`RestartSammy`:** reset scan (id 1000, radius 50000) restores gems, checkpoints and objects; hearts go back to 48 and `AttackCall=0`.
   - Teleport to `Player1Start` (checkpoint or spawn) facing `Player1AngleStart`.
   - Snap the camera to (+32,+32,+150), fade in alpha +10/tick, and restore camera parameters.

### 5.8 Shadow
- **When `AverageFps>40`:** the engine dynamic shadow.
- **Otherwise:** a flat sprite `ombblob2.pcx` (scale 0.8×1.2) at ground+0.5 (`player.wdl:195-215`).
- **Port:** use one blob shadow.

---

## 6. Camera (`camera.wdl`)

### 6.1 Per-level parameters (`InitCamera1-4`, `mm1.wdl:2719-2851`; defaults `InitLvl.wdl:~140-175`)
- **`InitCamera1`:**
  - Skill1 height `HautCamPerso` (default 800), Skill2/3 min/max height (150/1200).
  - Skill4 distance `PersoDistCam` (400), Skill5/6 min/max (50/600).
  - Skill7 angle behind `PersoAngArriere` (180), Skill8 look-at offset `Camera_PlusTilt` (200; **always applied, even 0**).
  - Flag1 camera may move in Z.
  - Camera type: Flag2 → **type 3** (standard), Flag3 → type 1, Flag4 → type 2.
- **`InitCamera2`:** frictions.
  - Skill1-3: horizontal friction for types 1/2/3 (type 3 default 4).
  - Skill4-6: vertical friction (0.1 / 0.05 / 0.01).
  - Skill7 → arc (but reads Skill6) and Skill8 → ambient (but reads Skill7): off-by-one bug, replicate.
- **`InitCamera3`:** Flag1/2 make doors solid/opaque to the camera; Skill1-4 3rd-person limits; Skill5 ground clearance (16); Skill6 `Camera_LookZ`; Skill7 retrieve friction (0.7); Skill8 start friction (0.5).
- **`InitCamera4`:** zoom minimum height (105), PlusTilt minimum (105).
- **`InitGame2`:** level-start camera origin, pan and tilt. **`InitGame4`:** mip settings and `Clip_RangeGame` (×1.5 at runtime). **`InitGame7`:** fog number and intensity.

### 6.2 Type 3 "Suricate" follow camera (`HandleCamera3 :384`, `CamFollowMe :442-823`, `LookAtTarget :825-873`)
1. **Yaw follow:** `AngCamPerso += Ang(pan - AngCamPerso) × 4/AverageFps`.
2. **Ideal position:** `PersoDistCam` behind (rotated by `AngCamPerso+PersoAngArriere`), at height `player.z + HautCamPerso (+16 if close to ground)`.
3. **Camera proxy:** `Camera_Syn` (a fat-hull invisible entity) moves toward the ideal with factors `min(4×AdjustFpsCam3×time,1)` horizontally and `min(0.01×AdjustFpsCam3×time,1)` vertically (very slow vertical follow unless a level sets it).
   - `AdjustFpsCam3` = 0.5, or 0.8 below 20 fps, or 1 below 12 fps.
   - If the proxy hits objects more than 12 times, it becomes passable.
4. **Visibility trace** from the player's eye (z+PlusTilt) to the ideal position, ignoring passable entities, sprites and models. Doors (id 90273) are see-through while `DoorInvisible==1`.
   - **Clear:** `camera = Camera_Syn`.
   - **Blocked:** switch to the **emergency camera** `Camera2_Syn`. It targets a point 32 short of the wall hit along the last good yaw. If that is also blocked, it **zooms in**: distance ×(1-0.75·time) down to 50, height -50/tick down to 105. The look-at drops toward `PlusTiltMin`. It returns to normal when the main ray clears and they converge (Δ<2) or after 80 ticks.
5. **Look-at:** pan snaps; tilt eases 0.4 (normal) or 0.8 (emergency) ×mintime toward the target, clamped ≤ +20°. A distance more than 3.5× `PersoDistCam` forces emergency mode.
6. `Camera.diameter=64`. Look-at Z eases 1/4 per frame.
7. **Remote-controlled wasp:** if a type-6 wasp is flying, holding Alt or pushing forward views from the wasp.

### 6.3 Types 1 and 2
- **Type 1** (`HandleCamera1 :278`): fixed offset `BlobCamDelta` from the player. Friction 1 horizontal / 0.05 vertical. No rotation.
- **Type 2** (`HandleCamera :13`): chase camera with optional fixed height. Friction 1/0.1; keeps at least 80 above the floor.
- C cycles types (debug). Mouse orbit code exists but is dead (`Cameramouse=0`).

### 6.4 Camera zones (`SetCamera :958`, `SetCameraHit :1002`, `CameraSurveillance :1175`)
- **Trigger:** the player's id-25 scan, with line of sight and distance ≤ Skill7.
- **Transition:** the camera type becomes Skill1. Distance, height, back angle and PlusTilt interpolate linearly to Skill2/3/4/5 over Skill6 ticks, with frictions capped at 0.25 during the transition.
- **Surveillance shot** (Skill8≠0): the camera moves to a fixed point (Skill2-4 or the zone itself) and only looks at Sammy.
- Flag2 keeps the camera locked afterwards.

---

## 7. Gems physics summary (`gems.wdl`)
- **Idle:** animate frames 1-17 at `Skill31` (3)/tick; snap to `ground+HeightGem(25)` unless Flag6. Gravity -2/tick (Flag7).
- **Pickup:** shrink ×0.8/tick, `ColoredGem_Son` (gemding2), a coloured light of range 300 fading 15/tick, and `particle_GemCatch*` bursts (5/10/10).
- **Thrown-back gem:**
  - vertical speed 10, friction 2.5;
  - horizontal speed = 2× player speed, decaying 0.5/tick;
  - bounce sounds ding6/5/3/7;
  - catchable after the 5th bounce;
  - waits up to 16 ticks while overlapping another gem (scan id 101).
- **Mix animation:** two gems spiral out to radius 80 and back at 70°/tick, then a burst of 70 `particle_Explose*` with EndGemMixing_Son.
- **Crush** (scan id 100 from lions with Flag6): explode; return after Skill3 ticks if set.

---

## 8. Enemies: lions, scorpions, elephant (`lion.wdl`, `anims.wdl:2-350`)

### 8.1 Types (`InitLion :11-236`; `int(Skill1)`)

| Skill1 | Model | Class id | Weight | Head vertex | Default resistance (HP in wasp-poison units) |
|---|---|---|---|---|---|
| 10 | nanolion | 8888 | 0.65 | 109 | 6 |
| 0 / 11 | rasta | 8888 | 0.62 | 135 | 9 |
| 12 | hippie | 8888 | 0.60 | 132 | 12 |
| 13 | rockerA | 8888 | 0.55 | 139 | 15 |
| 14 | punk | 8888 | 0.50 | 209 | 30 |
| 2 / 3 | scorpion (skin 1/2) | 8888, spec 3 | 0.75 | n/a | 12 |
| 1 | elephant (no model shipped) | 1111 | 0.30 | n/a | 100 |

### 8.2 Packed editor fields
- `Skill1` fraction ×100 = resistance override.
- `Skill2` int = speed multiplier, fraction ×100 = bounce force (default 2).
- `Skill3` = attack range (default 500); fraction ×10000 = second attack range.
- `Skill4` = path id (patrol, Flag3) with fraction ×10000 = path radius, or random-mode timer with fraction ×1000.
- `Skill5` int = lives (respawns), fraction ×10 = run multiplier `LionAccelAttack`.
- `Skill6` = react range (default 1000); fraction ×10000 = second react range.
- `Skill7` = respawn delay.
- `Skill8` = "time for nothing".
- Flags: Flag3 patrol, Flag4 **no attack**, Flag5 does door/camera scans, Flag6 crushes gems, Flag7 calls friends, Flag8 flies.

### 8.3 States (`LionEtat`)

| State | Meaning |
|---|---|
| 0 / 4 | idle (stand / alt-stand) |
| 1 | walk / patrol |
| 2 | chase |
| 3 | roar |
| 6 | lethal hit, knockback |
| 7 | death animation |
| 8 / 9 | random-mode idle that can attack |
| 10 | swipe |
| 11 | hit reaction |
| 12 | flee |
| 13 | random run |

### 8.4 Behaviour (`EnnemyBehavior :1678`)
- **Activation:**
  - idle → walk when the player is within the react range;
  - walk/patrol → **chase** when line of sight is confirmed (trace checked every 10 ticks, or every 4 while chasing; scorpions 16/8) **and** distance ≤ attack range **and** |elevation| < 45° **and** `random(100) > 50`.
  - Chase start plays LionAttack1 (lions, 50%) or ScorpionAttack, calls `Warning_Attack` (an orange light flash on Sammy), and switches to the second ranges.
- **Disengage:**
  - give up chasing beyond attack range + 128;
  - lose interest beyond 1.1× react range;
  - turn around if within 16 of the player.
- **Speed:** `SpeedEnnemy × time × BallAccel`, with ×1.5 when swiping and ×1.2 when fleeing. Rotation toward the destination pan at `SpeedRotationLion` (0.2 far / 0.02 near).
- **Patrol:** cone-scans path nodes (range = path radius), aligns to the node heading within 150, takes the next node within 100, and rescans after 64 ticks without progress.
- **Being hit by wasps:**
  - `resistance -= WaspPoison`, with knockback ×2 (×2.5 on impact);
  - the hit reaction plays when resistance falls below 95% of the original;
  - with Flag7, a roar at ≤75% resistance calls friends: three expanding `circl100.mdl` rings, and same-type lions within `RangeCall` start chasing;
  - fear accumulates (`EnnemyAttacked`) and causes **flee** above `|res × weight × 0.5|`;
  - at ≤0: death animation → `LionDeath`.
- **Death and respawn:** blink for about 40 ticks, `LionNbrVie -= 1`. If lives remain, wait until the player is farther than the attack range, then respawn at the origin with `SpeedEnnemy+1`. Resistance is not reset (bug), so a respawned lion dies to one wasp.
- **Damage to Sammy:** see §4.4.
- **Elephant:** never damages Sammy; only crushes gems and pushes others.
- **Wall avoidance:** a trace of `DistMuzzle=32` ahead of the head vertex acts like a wall hit. On a wall the lion turns to the wall normal ±25-45°.
- **Gravity:** -4 with accel `AccGraviteEnnemy=3`. A lion more than 200 above ground with the player 100-400 above it jumps up (vz 20).
- **Range rings** (with Eye of Truth): `circl100.mdl`, skin by alert level, scale = attack range/1000.

### 8.5 Animations
Scenes: stand, AltStand, walk, run, rugit, attack, Hit, death. Speeds: walk 0.82, run 0.3, stand 3, roar 4, death 4, attack 6, hit 10, multiplied by `SpeedEnnemy`. Transitions cross-fade through `LionAnimPreviousStep`. Hit sounds: LionHit1/2 or Tigre.

---

## 9. Wasps (`bullets.wdl`)

### 9.1 Ammo pickup (`InitAmmo1 :21-179`, `wasp1.mdl`, skin = type)
- **Fields:** Skill1 type 1-7; Skill2 ammo per pickup (1 typical [D]); Skill3 poison.
- **Defaults by type:** poison {3,2,1,1,4,6,1}; scales {—, 0.8, 0.65, 0.65, 0.8, 0.8, 0.65}. Flag3=0 → overlay, Flag3=1 → flare.
- **Idle:** slow spin and stand animation.
- **Picking up a different type:** up to 15 "escaping" wasps of the old type fly off, and ammo resets to 0. Then `NbrAmmo1 += Skill2`, with clog3 + TakeWasp sounds and a light in the type colour.

### 9.2 Fired wasp (`InitWasp :203`, `Wasp :348`)
Each fired wasp lives 128 ticks (type 6: 200) and deals `WaspPoison` = `WaspCurrentPoison`.

| Type | Colour | Scale | Speed/tick | Homing | Special |
|---|---|---|---|---|---|
| 1 queen | orange | 1.0 | 32 | scan 500, tilt gain 0.1 | overlay |
| 2 | yellow | 0.7 | 100 | n/a | n/a |
| 3 | blue | 0.5 | 100 | n/a | 3-wasp fan |
| 4 | green | 0.5 | 100 | 500 / 0.1 | 3-wasp fan |
| 5 | magenta | 0.7 | 36 | 700 / 0.3 | climbs 3 ticks, levels 5, then dives |
| 6 | red | 1.0 | 38.4 | 1000 / 0.2 | **remote-controlled**: steer with left/right (`Pan -= Intention.x×20×time`); the camera can follow it |
| 7 | white | 0.5 | 100 | n/a | n/a |

- **Homing:** steering eases pan 0.4×Δ and tilt by gain×Δ toward the target's head vertex.
- **On contact with a wall or entity:** the wasp dies. Wasps also open Flag4 doors and break weak doors.
- **Death:** WaspKaputt1/2/3 sound, `particle_WaspExplode*` (or `particle_hit1` bursts when it hit a lion), then the corpse tumbles with the "death" animation.
- **Trails:** `particle_Wasp<colour>` every 2-4 ticks.

### 9.3 Hives (`InitHive :1197`; levels 176g_t, 177e_j)
- Emits bouncing wasp pickups every Skill8 ticks while the player's ammo is below the cap (Skill3 fraction ×100) or of another type. When hit by the player it emits toward him.
- Total output = int(Skill3); afterwards the hive explodes into 6-colour particles.
- Emitted wasps settle and become pickups worth 1 ammo, with an optional lifetime.
- **Bug:** fate value 1001 ends the action immediately (`bullets.wdl:1007`).

### 9.4 Wasp-enemy clone (`InitWaspEnnemy :1502`)
A lion-AI clone wearing the wasp skin. It is used only in `extro.wmb`; id 99 is the "You Win" decoration.

---

## 10. Sounds and music (`sons.wdl`) [R]
`play_entsound`'s third argument is the audible range [I].

| Sound (file) | Where |
|---|---|
| beep1, beep2 | debug, locked level click |
| Ding1 (tocbois8) | menu click |
| Ding2 (tocbois6) | options clicks |
| Toc2 (cligno1) | menu hover |
| Ding3/5/6/7 | gem bounces |
| Gagne1 | small heart, extra life |
| Error1 | no ammo |
| Chimes2 | boost, heart |
| clog3 | wasp pickup, popups, video slider |
| broken4 | gem explode |
| tigre | lion roar, sfx test loop |
| LionAttack1, LionHit1/2 | lions |
| Wasp_son | wasp launch |
| TakeWasp | wasp pickup |
| Bell1/Bell2 | door timer, titles |
| DoorOpen/DoorClose | doors |
| Hit1/2/3 | swipes, weak doors |
| Kaputt | death |
| ColoredGem (gemding2) | key pickup |
| Diam | diamond |
| GemMixing / EndGemMixing | mixing |
| ScoreReach (applause) | goal reached |
| Platform1 (platfor2) | platform unlock |
| WaspKaputt1-3 | wasp death |
| ScorpionAttack / ScorpionWalk | scorpions |
| Win (Iwin) | exit tile |
| GetLife | big heart |
| Heart_Away | life lost |
| plank1/2 | weak doors |
| Restart | checkpoint |
| Blouit | U-turn, stomp |
| EyeTruth | eye pickup |
| cligno1 | gem blink |
| glisse | slope slide (looped and retuned) |

Unused: Toc1, smash1, Frot1.

**Music:**
- `musique1-7` = `sammy01-07.mid`; `musique8/9` are duplicates of 01/02.
- Menu and intro: song 1.
- Levels: `MusicOrder[level]` (§3.1).
- Options preview: song 4.
- The level-end screens continue the current song.

---

## 11. HUD and panels (`panels.wdl`, `paninter.wdl`, `interfac.wdl`) [R]
- **Resolution model:** layouts are defined per video mode (2, 5-9). Positions for 640×480 (mode 6) are below; other modes are in `panels.wdl:806-1025`. The **port should anchor and scale from 640×480.**
- **The HUD is shown only when `TypeJeu==25`** (`InitPanels :640-805`).

| Element | Bitmap / font | Pos (640×480) | Binding |
|---|---|---|---|
| Lives background | fondlife.pcx 88×46 | 12,2 | n/a |
| Hearts bar | vbarhear.pcx, `WINDOW -12,0,12,48` scrolled by `WindowHearts` | in PanLife | health 0-48 |
| Lives digits | boink22 font, 2 DIGITS (factor 1 and 0.051 = tens) | 2,2 (+60,20 / +44,20) | `BallesJoueur1` |
| Score background | fondscor.pcx 177×51 | 464,4 | n/a |
| Score / goal digits | boink24 at 55,3 ×5; lucidd14 at 35,32 ×5 | 464,4 | `ScoreJoueur1`, `ScoreGoal` |
| Gem background | fondgems.pcx 103×89 | 268,5 | shown when a key is held |
| Current key icon | hbargems.pcx 336×42 strip, 42×42 window offset `KeyPlayer1×42` | 298,29 | `playerKey` (slot 7 = master) |
| Boost bar | Hbarboos.pcx `WINDOW 50,20,150,8` offset `WindowBoost`, frames panboost/panboos2 | 0,418 / 46,400 / 6,400 / 12,400 | shown when not empty |
| Ammo digits | boink22 | 540,433 | `NbrAmmo1` |
| HUD wasp | 3D `panwasp.mdl` in camera space, skin = type, spinning 2°/tick | n/a | `WaspCurrentType` |
| Eye of Truth | 3D `Paneye.mdl` | n/a | shrinks when it expires |
| Fade | 3D `Black.mdl` quad (`PanBlack`) | n/a | transitions |
| Popups | world sprites `plus50+6`, `plu100+2`, `extrabal`, `Nextraba` (FlyingPCX: rise at 40/tick, about 32 ticks) | n/a | n/a |

- **Fonts:** bitmap grids (`fontes.wdl`, `SamTexts.wdl:2-3`):
  - `ackfont` 6×9, `lucida38` 25×38, `boink38` 24×38, `boink22` 16×22, `lucidd14` 15×14, `boink20` 14×20, `boink24` 17×24, `boink19` 15×20;
  - Polish fonts `polish1320` 13×20 and `polish1017` 10×17, with CP1250 glyph layout.
- **Unused panels:** the pre-level "Ready / score to reach" panels (`InitPanelsPret`) are disabled. `PanScore2`, the best-score display and the end-of-world panels (4×4 placeholders) are dead.

---

## 12. Particles (`particul.wdl`) [R]
- **Emission:** `EMIT n,pos,action`.
- **Particle fields:** `MY_AGE` (ticks), `MY_SPEED` (added each frame), `MY_SIZE` (scale unknown; tune about size/1000 world units [I]), `MY_COLOR` (0-255), `MY_MAP` (bitmap), `MY_FLARE` (additive). There is no alpha; fading is done by shrinking the size or swapping the bitmap.
- **Live effects:**

| Effect | Look | Life |
|---|---|---|
| `particle_rouge/jaune/orange/Bleu/magenta/Vert` | coloured dots, speed ±1, z -4..-3, size 100-200 | 10 |
| `particle_*Bmp` | star sprites `tistar{1-6}{a,b}`, ±3, size 750-1050 | 10 |
| `particle_GemExplose*` | `morgem` shards, speed V4×±3, z V4..V4+30, gravity 4/tick, size 2000+, leaves dots | 30 |
| `particle_GemCatch*` | stars, z +1..+21 rising, gravity 1, size -50/tick | 30 |
| `particle_Explose{Rouge,Jaune,Bleu}` | normalized burst, gravity 1.5 (gem mix end) | n/a |
| `particle_ExploseDiam` | white stars, size -30/tick | 8 |
| `Particle_GemKeys` | trail of 3 stars every 3 ticks | n/a |
| `particle_GemRestart` | green, rising, xy ×0.9 | 30 |
| `particle_TileDeath` | rising smoke through `fumeeB*`/`fumee*` frames, growing to 2500 | 22 |
| `particle_Poussiere1` | brown dust `fumeeC*`, gravity 0.08/frame | 40 |
| `particle_SpeedySammy` | red `StarHit`, falling | 24 |
| `particle_Boost1` | orange star from boost model vertices 3 and 11 | n/a |
| `particle_Teleport` / `Teleport2` | rising StarHit | 20 / 16 |
| `particle_life` | heart frames `phearth1-7` | n/a |
| `particle_Getlife` | spawns `particle_life` | n/a |
| `particle_SmallHeart` | n/a | 12 |
| `particle_RangeLion1/2` | eye-of-truth ring dots | n/a |
| `particle_TakeEyeTruth` | n/a | n/a |
| `particle_hit1` | red StarHit burst | 16 |
| `particle_Wasp*` | trails, 4-frame `WaspEf*` loop, size 3000→100 | 18 |
| `particle_WaspExplode*` | n/a | 18 |
| `particle_interface1` | menu sparkles | n/a |
| `Particle_WaspHive` | n/a | n/a |
| `Particle_Win` | 6 stars every 3 ticks | n/a |

- Many effects are thinned out at low fps; use the high-fps values.
- **Dead effects:** smoke emitters (`InitFumee` chain), ball/rocket trails, `EmitParticle2/3`, `particle_test*`, `particle_explose3/4`, bubbles, JackPot.
- **Name clash:** `particle_rouge` (:117) and `particle_Rouge` (:476) collide (case-insensitive).

---

## 13. Global game-state variables (key subset; full list in `skills.wdl`)

| Variable | Meaning |
|---|---|
| `LevelEnCours` | current level 1-49 |
| `LevelsInfo[]` / `LevelsExist[]` | unlock state |
| `BestScores[]` | best score per level |
| `Player1RestartX/Y/Z[]` | checkpoint per level |
| `NbrLevelsReach` | levels reached |
| `PartieEncours` | 1 while a level runs; all loops test it |
| `PartieGagnee` | level won |
| `ChoixMenu` | menu choice; 999 = abort |
| `MenuOptionsGame` | 0 menu / 1 game / 2 extro |
| `ScoreJoueur1`, `ScoreGoal` | score and goal |
| `BallesJoueur1`, `MaxBalles=19`, `NbrBallesStart`, `NbrNextraBall1..24` | lives, cap, start lives, carry-over pools |
| `WindowHearts` (48) | health |
| `WindowBoost` (0 full … 150 empty), `ConsoBoost=1` | boost bar and cost |
| `KeyPlayer1` / `Pov_Syn.playerKey` / `playerOldKey` | key held / key in flight; 999 = no pickups |
| `NbrAmmo1`, `WaspCurrentType`, `WaspCurrentPoison` | wasp ammo |
| `EyeOfTruth`, `DureeEyeTruth` | Eye of Truth state and timer |
| `PlayerInvincible` | 1 normal, -1 invincible |
| `PlayerMoveMode` | 0 = no input |
| `Facilite` | loss counter; no consumer |
| `ScanIdentite` | broadcast tag (§`wdl-porting.md` 3.1) |
| `Singer`/`Siger` | registration flag (treat as 1) |
| `ExtroVue`, `BonusState`/`BonusActif` | extro seen; bonus availability |
| `AverageFps`, `ticks1` | fps measurement and frame counter, used for "every N frames" logic |

**ScanIdentite values in use:**

| Id | Use |
|---|---|
| 1 | pickup |
| 2 | wasp seek |
| 21 | remove nearby object |
| 25 | door / camera zone (and the extro lion Flag5) |
| 27-99 | lion path ids |
| 30 | intro path |
| 100 | crush gems |
| 101 | gem overlap test |
| 102 | door clearance |
| 1000 | reset after death |
| 2222 | Bonus5 link |
| 8465 (or the platform Skill1) | platform link |

---

## 14. Bugs, quirks and dead code worth knowing (all [R])
Replicate any that affect level tuning; fix purely cosmetic ones at will.

1. **`InitGame6`:** `RangeCall=Skill7` should be Skill8 (`mm1.wdl:1950`).
2. **`InitCamera2`:** arc and ambient are read off-by-one (`mm1.wdl:2818-2824`).
3. **`InitSkyMenu`:** tests Skill5/6/7 but rotates by Skill2/3/4.
4. **`moteurBonus_Hit`:** sets Roll from Tilt (`mm1.wdl:2603`).
5. **`DoorLight`:** reads stale `V` (`mm1.wdl:3261`). `InitDoor` overwrites the editor Skill1 with 90273, the "camera-transparent" tag.
6. **`SetAnimPlayer`:** re-arms with the `SetPovSyn` event and uses Skill5 as a counter (`mm1.wdl:5522`).
7. **`TileDeath_Hit`:** sets `LionEtat=6` and then `1004`.
8. **Lion respawn:** resistance is not restored (`lion.wdl:704`).
9. **`Tune_Sound`:** gets the trace result instead of the sound handle (`lion.wdl:~1945`). Port: play the lion roar with volume `30000/dist` and pitch `200×weight±25`.
10. **Exact-float tests:** gem blink `Scale==1.5`; lion jitter `Int(GuardStuckBalle==8)`.
11. **`ForceChute`:** zeroed before being read in two places, so a hard landing triggers only from the airborne landing branch (`player.wdl:1273-1276, 1601`).
12. **`Player_Kaputt`:** sets hearts to 0 even when the player is invincible.
13. **HUD ×57:** `InitLvl.wdl:68` uses `KeyPlayer1*57` for the HUD gem window; elsewhere it is ×42 (`WidthGemPanel`).
14. **`InitPanelsPret`:** misses levels 32 and 42 (dead screen anyway).
15. **Yes/No dialog:** Y/O keys don't work in the Polish build (`language=1`).
16. **Untranslated English strings** remain: slow-machine warning, NOLEVEL, registration texts, "GAME OVER", "Hint : Yellow is the key".
17. **Shipped cheats and console** (§5.1); DRM entities (`Initpublic`/`Initprotection`).
18. **Bonus levels:** cut (missing WMBs), but the menu and unlock code still handle them.
19. **Help page 2 bitmap:** `small.pcx` (blank).
20. **Mouse:** mouse camera control is dead.
21. **`TeleportFor1Life`:** only reachable from death tiles (§4.4). Grepping `player.wdl` alone makes it look dead.
