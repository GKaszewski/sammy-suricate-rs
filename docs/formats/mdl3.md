# MDL3 / MDL5 — Acknex vertex-animated models

Parser: `crates/formats/src/mdl.rs`. Converter: `sammy-convert model <out> <files...>`.

Conitec's A4 (`MDL3`) and A5 (`MDL5`) model format, derived from Quake 1 MDL
(`IDPO` v6). All 37 models in Sammy Suricate parse with every byte accounted for
(the parser rejects trailing data). 32 files are `MDL3`; five 2D logo/panel
quads are `MDL5` (`Lose`, `Win`, `titre`, `surilogo`, `publogo`).

All integers are little-endian. Coordinates are native Acknex: X forward, Y left,
Z up (right-handed), in quants.

## Header (0x54 bytes)

| Off  | Type      | Field            | Notes |
|------|-----------|------------------|-------|
| 0x00 | char[4]   | magic            | `MDL3` or `MDL5` |
| 0x04 | u32       | ?                | always 0 (Quake: version) |
| 0x08 | f32[3]    | scale            | per-axis; may be 0 for flat models (e.g. `Black.mdl` x = 0) |
| 0x14 | f32[3]    | translate        | origin; `pos = scale * packed + translate` |
| 0x20 | f32       | bounding radius  | |
| 0x24 | f32[3]    | eye position     | always 0 here |
| 0x30 | i32       | num_skins        | |
| 0x34 | i32       | skin_width       | MDL5: 0 (each skin has its own size) |
| 0x38 | i32       | skin_height      | MDL5: 0 |
| 0x3c | i32       | num_verts        | xyz vertices per frame |
| 0x40 | i32       | num_tris         | |
| 0x44 | i32       | num_frames       | |
| 0x48 | i32       | num_skin_verts   | UV vertices (independent of xyz count) |
| 0x4c | u32       | flags            | always 0 here |
| 0x50 | u32       | ?                | always 0 (Quake: `size`) |

Quake's `synctype` is gone and `num_skin_verts` sits in its place.

## Skins (`num_skins` times)

| Type | Field |
|------|-------|
| u32  | skin type |
| i32, i32 | width, height (**MDL5 only**) |
| …    | `width * height` pixels, row-major, top row first |

Skin types seen:

| Type | Pixel format | Size | Used by |
|------|--------------|------|---------|
| 0 | 8-bit palette index | 1 B | `cliprang`, `spheran1/2`, `spherang` (16×16). No palette in the file; the parser returns a grayscale stand-in plus raw indices. These look like clip/helper models. |
| 2 | RGB565 | 2 B | almost everything (MDL3 and MDL5) |
| 3 | ARGB4444 (`a = p>>12, r = p>>8&15, g = p>>4&15, b = p&15`) | 2 B | MDL5 `Lose`, `Win`, `titre` (alpha-cut logos) |

No skin groups (Quake type 1) and no mipmap variants appear. Any other type is
rejected by the parser.

## Skin vertices (`num_skin_verts` × 4 bytes)

`i16 u, i16 v` in skin **pixels** (origin top-left). There is no Quake `onseam`
flag. The parser normalizes with `u / width`, `v / height` (MDL3: header size;
MDL5: first skin size). Some values equal the width/height exactly (e.g. 16 on a
16-px skin), so no half-texel offset is applied. Most models store 3 UVs per
triangle (`num_skin_verts == 3 * num_tris`).

## Triangles (`num_tris` × 12 bytes)

`u16 xyz[3], u16 uv[3]`: indices into frame vertices and skin vertices separately.
No Quake `facesfront` field. Because the xyz and UV indices differ, renderers
need one vertex for each unique (xyz, uv) pair (`Mdl::vertex_buffer`).

**Winding:** as in Quake, a triangle's visible side is the side from which it
appears **clockwise** in the right-handed Acknex frame. So `(b-a)×(c-a)` points
*into* the model. This was confirmed by rendering: with CCW culling the meerkat's
belly and vest disappear; with CW culling they render correctly. The sky domes are
the other way round and are visible from inside. The flat logos (`titre`,
`surilogo`, …) face −X, so they read correctly to a camera looking along +X.
`vertex_buffer()` reverses the order to CCW for glTF/Bevy.

## Frames (`num_frames` times)

| Type | Field |
|------|-------|
| u32  | frame type: 0 = 8-bit vertices, 2 = 16-bit vertices |
| vert | bbox min (packed vertex, same encoding as below) |
| vert | bbox max |
| char[16] | name, NUL-padded (e.g. `walkB01`, `Frame 1`) |
| vert × num_verts | vertices |

Packed vertex:

* type 0 (all MDL3 files): `u8 x, u8 y, u8 z, u8 normal_index` (4 bytes)
* type 2 (all MDL5 files): `u16 x, u16 y, u16 z, u8 normal_index, u8 pad(0)` (8 bytes)

Decoding is `pos[i] = scale[i] * packed[i] + translate[i]`.

`normal_index` indexes Quake's 162-entry `anorms.h` table (max index seen: 161).
Per-vertex average face normals computed with `(b-a)×(c-a)` match the table
(mean dot ≈ 0.94 on suricat1, 0.95 on nanolion, 0.99 on sky2). Because that cross
product points inward (see Winding), the stored normals also point **inward**.
`Frame::normals` negates them so they point outward. Raw indices are kept in
`Frame::normal_indices`.

No frame groups (Quake type 1) appear. Any other frame type is rejected.

## Animation naming

Frame names are `<anim><number>`. Scripts address animations by the prefix
(`anims.wdl`: `set_cycle my, LionWalk_str, step`, with `LionWalk_str = "walk"`).
The step is a 0..100 percent of the whole cycle, advanced by
`AnimSpeed * time` per frame, where `time` is in ticks of 1/16 s. Acknex
interpolates between neighboring frames (`my.frame` is fractional, and
`next_frame` is used for blends). Matching ignores case: the scorpion's frames
are `Stand…` but the shared lion AI asks for `"stand"`.

`Mdl::animations()` groups frames by name with trailing digits and whitespace
stripped, in order of first appearance. Groups need not be contiguous:
suricat1's `toto` frames are split.

Speeds defined in the WDL scripts (percent per tick, often multiplied at runtime
by entity speed). Cycle length in seconds = `100 / (speed * 16)`:

| Character | anim string → speed |
|-----------|--------------------|
| Player (suricat1) | `stand` 2, `walkB` 0.5 (× velocity), `TirA`/`TirB` 20, `sautA` 30, `sautB` 14 (fast 30), `sautC` 14 (fast 18), `Turn` 14, `kaputt`/`Death` 3, `loopjambes` 2, `salut` 3, also `finkaputt`, `Iwin` |
| Lion family (nanolion, hippie, Punk, rasta, rockerA, scorpion) | `stand` 3, `walk` 0.82, `run` 0.3, `rugit` 4, `death` 4, `attack` 6, `Hit` 10, `AltStand` |
| Wasp (wasp1/panwasp) | `stand` 3, `walk` 0.5, `run` 1.1, `death` 6, `attack` |

Exported glTF animations use a fixed 16 frames per second (one frame per tick,
`model::FRAME_RATE`) with linear blending. The real speeds come from these script
constants. The node's `extras` include the frame names and the group→frame-index
map, so a runtime can drive frames itself.

## Models

| File | Ver | verts | tris | frames | skins | animation groups |
|------|-----|-------|------|--------|-------|------------------|
| suricat1 | 3 | 394 | 705 | 553 | 1 (120×480) | stand walkA walkAB toto walkB sautA sautB sautC tirA tirB turn death kaputt finkaputt Iwin loopjambes salut |
| wasp1, panwasp | 3 | 62 | 96 | 155 | 7 (92×64) | base walk walk_trans death stand |
| nanolion | 3 | 298 | 582 | 256 | 1 | stand AltStand rugit attack hit walk run death |
| hippie / Punk / rasta / rockerA | 3 | 432/514/465/444 | 804/868/852/820 | 256 | 1 (256×250) | same as nanolion |
| scorpion | 3 | 319 | 396 | 169 | 2 | Stand Rugit Hit Attack Walk Run Death |
| circl100, grate1, plank1-3 | 3 | 56/18/8 | 56/16/12 | 198 | 3/1/1 | Quake-player set (stand run attack pain jump flip salute taunt wave point crstnd crwalk crattak crpain crdeath death) — apparently built from a Quake-player template |
| boost, cactus1, huttop2, lifeA | 3 | 16/92/49/370 | 24/158/80/106 | 30/21/21/11 | 1/1/2/1 | stand |
| bigtree1 | 3 | 365 | 656 | 2 | 1 | stand toto |
| sky2-4 / sky5 | 3 | 365 | 656 | 1 | 1 (756×256 / 768×512) | stand |
| greenhi1, solmenu4/5 | 3 | 710 | 1254 | 1 | 1 (352×349) | stand |
| cliprang, spheran1/2, spherang | 3 | 62/76/91/124 | 64/56/70/128 | 1 | 1 (8-bit 16×16) | stand |
| Black, Paneye | 3 | 4 | 2 | 1 | 2/1 | `Frame 1` |
| Lose, Win, titre | 5 | 6 | 4 | 1 | 1 (ARGB4444) | stand |
| surilogo, publogo | 5 | 6 | 4 | 1 | 1 (RGB565) | stand |

## Unknowns

* Header 0x04 and 0x50, `flags` and the eye position are always zero, so their
  meaning can't be confirmed from these files.
* There is no palette for type-0 (8-bit) skins. A4 probably used the level or
  engine palette. They only appear on 16×16 helper models.
* The game's actual frame interpolation speed for each animation depends on
  scripts (see above). There is no rate stored in the file.
* The 16-bit vertex pad byte is always 0.
