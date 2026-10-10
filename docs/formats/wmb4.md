# WMB4 — Acknex A4 level / map-entity format

Used by Sammy Suricate for all levels (`levels/*.wmb`) and for brush-based entity
models (doors, platforms, boulders, menu titles…, `extracted/*.wmb`). It is a
Quake BSP29 derivative compiled by WED/WMP2WMB. Parser: `crates/formats/src/wmb.rs`;
exporter: `sammy-convert level` (`crates/sammy-convert/src/level.rs`).

All values little-endian. Coordinates are native Acknex: X forward, Y left, Z up,
units = quants. Angles in degrees (pan, tilt, roll).

Status: all 133 shipped WMB files parse; every lightmap size is reproduced
exactly; rendered geometry/lightmaps verified visually (see "Verification").

## Header

| off | size | field |
|-----|------|-------|
| 0   | 4    | magic `"WMB4"` |
| 4   | 16×8 | lump directory: `u32 offset, u32 length` (absolute file offsets) |

Lumps are not stored in index order. An empty lump has length 0 (offset may be 0).

| # | name | record size | notes / difference from Quake |
|---|------|-------------|-------------------------------|
| 0 | palette/tables | — | 181520 bytes in 27 levels, else empty. See below |
| 1 | planes | 20 | as Quake: `f32 normal[3], f32 dist, i32 type` |
| 2 | textures | var | 16-bit RGB565 (or 8-bit) miptex, see below |
| 3 | vertices | 12 | `f32 x, y, z` |
| 4 | visibility | var | Quake PVS, RLE (0 byte + count of zero bytes) |
| 5 | nodes | **40** | see below |
| 6 | texinfo | **64** | see below |
| 7 | faces | **24** | plane index moved to the end, see below |
| 8 | lightmaps | var | **RGB565**, 2 bytes/luxel, see below |
| 9 | clipnodes | **16** | `i32 plane, u32 junk, i32 children[2]` |
| 10 | leaves | **36** | see below |
| 11 | marksurfaces | 4 | **u32** face indices |
| 12 | edges | 8 | **u32** `v0, v1`; edge 0 is an unused dummy (may contain garbage) |
| 13 | surfedges | 4 | i32, `>=0`: edge v0→v1, `<0`: edge `-i` reversed |
| 14 | models | **160** | always exactly 1 model (the world); see below |
| 15 | entities | var | binary object list, see below |

"junk" fields contain stale in-memory pointers from the compiler (e.g.
`0x035C1120`) or 0 in small files; they carry no information.

## Lump 0 — palette / lookup tables

Present in 27 of the levels (`10007..10012`, most `1xx_*`, `extro`), identical
between some levels. Starts with something palette-like (768 bytes) followed by
large lookup tables (looks like colour/translucency maps for the 8-bit software
renderer). It is **not** the palette of 8-bit textures (decoding `#default` with
it produces noise). Kept raw as `Wmb::palette_lump`; unused.

## Lump 2 — textures

```
u32 count
u32 offset[count]        // relative to lump start
per texture (at offset):
  char name[16]          // NUL-padded, e.g. "Sand5_256x256"
  u32  width, height
  u32  mip_offset[4]     // relative to texture start; mip0 is always 40
  pixels for mips 0..3
```

Pixel depth is implied by `mip_offset[1] - mip_offset[0]`:

* `== w*h*2`: **16-bit RGB565** (all real textures; verified visually: sand,
  stone, wood, door textures look correct).
* `== w*h`: **8-bit palettized**. Only WED's `#default` texture (5 files:
  `107i_j`, `114t_t`, `171c_t04`, `180c_t`, `setcam`). The palette is the
  standard Quake palette (WED's `palette.raw`; verified visually), which is not
  part of the game data. `parse` decodes these as greyscale;
  `parse_with_palette` takes a palette, and `sammy-convert level` uses a
  `palette.raw`/`palette.lmp` found next to the input or in its parent directory.

Only mip 0 is decoded. Sizes seen: 16..256 per side, non-square allowed.

## Lump 5 — nodes (40 bytes)

| off | type | field |
|-----|------|-------|
| 0  | i32 | plane |
| 4  | u32 | junk |
| 8  | i16[3] | mins |
| 14 | i16[3] | maxs |
| 20 | u32 | junk |
| 24 | i32[2] | children: `>=0` node, `<0` leaf `-(c+1)` |
| 32 | u32 | first face |
| 36 | u32 | face count |

## Lump 6 — texinfo (64 bytes)

| off | type | field |
|-----|------|-------|
| 0  | f32[4] | s vector + offset: `s = dot(p, v.xyz) + v.w` (texels) |
| 16 | f32[4] | t vector + offset |
| 32 | i32 | texture index |
| 36 | u32 | surface flags (below) |
| 40 | u16 | always 0 |
| 42 | i8  | surface ambient (−50…+70 seen; WED default 0) |
| 43 | u8  | surface albedo (30…100 seen; WED default 50) |
| 44 | 20 bytes | always 0 |

Texture UV = `(s / width, t / height)`; textures repeat.

Surface flags — meanings inferred from statistics over all files (WED 4 surface
modes; no engine code was decoded):

| value | inferred meaning | evidence |
|-------|------------------|----------|
| 1  | FLAT: no lightmap | every face with `light_offset = -1` has bit 0; no face with bit 0 has a lightmap |
| 2  | unknown | set on most flat prefab surfaces (planks, gems) and decorative brushes |
| 8  | **INVISIBLE** (not drawn) | only on `01dummy1`/`black`/`#default` clip boxes, outer level hulls, invisible posts, and 4 `lava2` faces (kill zone). Combinations seen: 9, 11, 27 |
| 16 | unknown (lit and flat) | e.g. stone/wood walls; perhaps "smooth" |
| 128 | unknown | once (`STONE005_128`, flags 131) |

Values observed: 0, 3, 9, 11, 16, 19, 27, 131. The exporter skips faces with
bit 8. No sky textures exist (the sky is drawn by WDL sky entities).

## Lump 7 — faces (24 bytes)

| off | type | field |
|-----|------|-------|
| 0  | u16 | unknown (mostly 3; 0..27 seen; **not** the plane) |
| 2  | u16 | side: non-zero → normal = −plane normal |
| 4  | u32 | first surfedge |
| 8  | u16 | surfedge count |
| 10 | u16 | texinfo |
| 12 | u8[4] | light styles (`0,255,255,255` lit; `255×4` unlit) |
| 16 | i32 | lightmap byte offset into lump 8, or −1 |
| 20 | u32 | **plane index** |

Winding: vertices are clockwise seen from the front (as Quake). 54432 faces
agree with the plane normal; 3 are reversed and 113 degenerate (tiny slivers).

## Lump 8 — lightmaps

RGB565 luxels, row-major, one rectangle per lit face at `light_offset`.
Rectangle size is computed exactly like Quake, **in f32**:

```
for each vertex: s = x*S0 + y*S1 + z*S2 + S3   (f32 ops in this order; same for t)
w = ceil(max_s/16) - floor(min_s/16) + 1
h = ceil(max_t/16) - floor(min_t/16) + 1
```

Using double precision gets ~5% of sizes wrong (values like −896.0001). Each
rectangle is padded to a multiple of 4 bytes (the last one may be unpadded). With
these rules the rectangles tile the lump exactly in all 38 files that have
lightmaps (39762 lit faces). Luxel `(i, j)` is centred at
`s = (floor(min_s/16) + i) * 16`. Luxel spacing is 16 *texels*, so world-space
luxel size depends on texture scale.

Typical unshadowed value is `0x730C` (≈ 45% grey), suggesting the engine
modulates with a 2× overbright factor; not verified. Prefab WMBs (doors etc.)
have no lightmaps (all faces FLAT).

## Lump 10 — leaves (36 bytes)

| off | type | field |
|-----|------|-------|
| 0  | i32 | contents |
| 4  | i32 | vis offset into lump 4, or −1 |
| 8  | i16[3] | mins |
| 14 | i16[3] | maxs |
| 20 | u8[8] | unknown (often repeats maxs.z as u16 at 20; leaf 0 holds junk) |
| 28 | u32 | first marksurface |
| 32 | u32 | marksurface count |

Contents values (leaves and clipnode children): **−256 empty**, **−128 solid**,
−240 (= −256|0x10, empty-ish, 66 leaves), −126 (= −128|2, 126 leaves / 408
clipnode refs; probably a special solid). Leaf 0 is the solid leaf.

PVS rows are `ceil(visleafs/8)` bytes; bit `i` = leaf `i+1`.

## Lump 14 — model (160 bytes, exactly one)

| off | type | field |
|-----|------|-------|
| 0   | f32[3] | mins |
| 12  | f32[3] | maxs |
| 24  | f32[3] | origin (0) |
| 36  | i32[4] | head nodes: [0] BSP node, [1],[2],[3] clipnode roots per hull |
| 52  | i32[4][3] | hull mins (hull0..3): `0 / −16,−16,−16 / −32,−32,−32 / 0` |
| 100 | i32[4][3] | hull maxs: `0 / 16,16,16 / 32,32,16 / 0` |
| 148 | i32 | visleafs (= leaves − 1) |
| 152 | u32 | first face (0) |
| 156 | u32 | face count (= all faces) |

There are no brush submodels: movable brushes are separate WMB files loaded as
entities (e.g. `file = "doorBlue.wmb"`). Hull 1 ("narrow") is ±16, hull 2
("fat") is 64×64×48. Hull 3 is unused (head 0).

## Lump 15 — entities

```
u32 count
u32 offset[count]     // relative to lump start, ascending; record size = next - this
records...            // each starts with u32 type
```

Types found: 3 (7266×), 2 (468×), 5 (44×), 1 (2×), 7 (1×, in `menu_greenpepper.WMB`).
No unknown types. Strings are NUL-terminated Latin-1 in fixed fields.

### Type 1 — position (56 bytes)

| off | type | field |
|-----|------|-------|
| 0  | u32 | 1 |
| 4  | f32[3] | origin |
| 16 | f32[3] | angles (pan, tilt, roll) |
| 28 | u32[2] | unused (0) |
| 36 | char[20] | name (`pos_001`) |

### Type 2 — light (36 bytes)

| off | type | field |
|-----|------|-------|
| 0  | u32 | 2 |
| 4  | f32[3] | origin |
| 16 | f32[3] | red, green, blue (percent, e.g. 80.39) |
| 28 | f32 | range |
| 32 | u32 | flags (always 0) |

Static lights; already baked into the lightmaps.

### Type 3 — entity (A4, 136 bytes)

| off | type | field |
|-----|------|-------|
| 0   | u32 | 3 |
| 4   | f32[3] | origin |
| 16  | f32[3] | angles: pan, tilt, roll |
| 28  | f32[3] | scale x, y, z |
| 40  | char[20] | name (`suricat1_mdl_248`, WED-generated) |
| 60  | char[13] | file (`suricat1.MDL`, `gemgr+17.pcx`, `doorBlue.wmb`) |
| 73  | char[20] | action (WDL action name, e.g. `initmarche`) |
| 93  | u8[3] | padding (0) |
| 96  | f32[8] | SKILL1..SKILL8 |
| 128 | u32 | flags |
| 132 | f32 | ambient (0, 50, 60, 80, 90, 100) |

### Type 7 — entity (A5 layout, 372 bytes)

| off | type | field |
|-----|------|-------|
| 0   | u32 | 7 |
| 4   | f32[3] ×3 | origin, angles, scale |
| 40  | char[33] | name |
| 73  | char[33] | file |
| 106 | char[33] | action |
| 139 | u8 | padding |
| 140 | f32[20] | SKILL1..SKILL20 |
| 220 | u32 | flags |
| 224 | f32 | ambient |
| 228 | f32 | albedo |
| 232 | i32 | path |
| 236 | i32 | entity2 |
| 240 | char[33] ×4 | material, string1, string2, unused |

### Type 5 — level info (56 bytes)

| off | type | field |
|-----|------|-------|
| 0  | u32 | 5 |
| 4  | f32[3] | origin (unused, 0) |
| 16 | f32 | sun azimuth (degrees) |
| 20 | f32 | sun elevation (degrees) |
| 24 | 32 bytes | always 0 (flags/version/gamma/lightmap size/colours in later versions) |

### Entity flags

Bits 0..7 = FLAG1..FLAG8 — confirmed by the scripts (e.g. `InitArbre`: flag1
→ facing, flag3 → nofilter; `InitDoor` uses flag5/flag7 matching the stored
0x40/0x50). Higher bits follow the later Acknex `atypes.h` order and fit the
data, but are not verified against the A4 engine:

| bit | value | name | seen on |
|-----|-------|------|---------|
| 8  | 0x100 | INVISIBLE | — (never set) |
| 9  | 0x200 | PASSABLE | grates, hut roofs, menu icons, sky meshes, `PosXxxLevels` markers |
| 10 | 0x400 | TRANSPARENT | 1 camera helper |
| 18 | 0x40000 | SHADOW | 4 cacti in the menu |

Values observed: 0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x8, 0x9, 0xe, 0x10, 0x14, 0x1c, 0x20, 0x30, 0x40, 0x50,
0x88, 0x8c, 0x9c, 0x200, 0x201, 0x20e, 0x21e, 0x240, 0x402, 0x40000.

## Exporter output (`sammy-convert level <out> <files...>`)

* `<name>.glb` — root node `<name>` with children: `<name>_world` mesh (one
  primitive per texture × lit/flat; TEXCOORD_0 = texture UV, TEXCOORD_1 =
  lightmap atlas UV, lightmap in the occlusion slot), then one empty node per
  entity (translation/rotation/scale converted with `glb::to_gltf`; rotation =
  `Rz(pan)·Ry(−tilt)·Rx(roll)` in native space, roll sign unverified) with all
  fields in `extras`.
* `<name>.entities.json` — same entity data in native coordinates, plus world bounds.
* `<name>.lightmap.png` — the lightmap atlas (shelf-packed, 1-luxel replicated
  border per face).

## Verification

* `cargo test -p formats --test wmb`: parses all 133 files, checks lightmap
  tiling, entity types, mesh index/winding sanity, point-contents and PVS
  decompression on 10001.
* All 129 output GLBs (4 prefab names are duplicated in `levels/`) re-parsed:
  buffer views/accessor counts/bounds/indices consistent.
* Software renders of 10001 (stone maze on sand with wall shadows) and 109o_t
  (castle with arches, towers, tiled floors) look correct.

## Unknowns

* Face u16 at offset 0.
* Texinfo flag bits 2, 16, 128; bit 8 = invisible is inferred.
* Leaf bytes 20..28; contents −240 and −126.
* Lump 0 contents beyond "palette + lookup tables".
* Entity flag bits ≥ 8 (names from later engine versions).
* Lightmap brightness scale (1× vs 2× overbright).
