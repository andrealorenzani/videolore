# Code — Apply LUT (1.6.0)

Back to the [index](code.md). Requirements REQ-135, REQ-136, REQ-137.

## Where things are

| Where | What |
|---|---|
| `src/lut.rs` | `.cube` parser (`parse`, `read`), `Lut` (a normalised 3D cube + hash), `cached(path)`, `path_ok`, `display_name` |
| `src/timeline.rs` | `clip_luts: Vec<ClipLut>` (since 1.6.1 there is no whole-video LUT: `FrameTarget::Whole` writes every clip's entry); `lut_of`, `set_lut`, `any_lut`; split/merge/remove rules; `DIFFERENT_LUTS` |
| `src/project.rs` | schema 8; `validate_luts` (damaged entries removed with a note, missing files named) |
| `src/render/` | `Segment.lut_whole` / `lut_clip`, `lut_for`, `lut_chain` (graph), `RenderError::Lut`, `FrameKey.luts`, the re-encode rule in `plan_export_choice` |
| `src/app/lut.rs` | `PickLut`, `SetLut`, `RemoveLut` commands, `missing_luts` |
| `src/app.rs`, `src/app/effects.rs` | `EffectKind::ApplyLut`, `Instance::Lut(FrameTarget)`, popup title, copy/cut/paste of a clip's LUT (`clipboard_lut`) |
| `src/ui/lut_popup.rs` | the popup body (file name, tooltip with the path, **Choose file…**, **Remove effect**) |
| `src/ui/markers.rs`, `timeline.rs` | one `MarkerKind::Lut` marker at a clip's start (the whole video has none) |
| `src/ui/actions.rs`, `menu.rs`, `effects.rs` | `ApplyLut`, `ApplyClipLut`, `ApplyWholeLut` actions; the Effects row and menus |

## The LUT file

`lut::parse` takes untrusted bytes and never panics: at most 16 MiB (`MAX_BYTES`), UTF-8 (a BOM is
accepted), `#` comments, the keywords `TITLE`, `LUT_3D_SIZE` (2..=65), `LUT_1D_SIZE` (2..=4096),
`DOMAIN_MIN`/`DOMAIN_MAX` (only the default 0..1; anything else is refused with a message), an exact
number of entries, finite numbers clamped to 0..1, no mixing of 1D and 3D. A 1D table becomes a 33^3
cube by per-channel linear sampling, so the graph has one code path. `Lut::to_cube()` writes the
normalised text and `Lut::hash()` fingerprints it (16 bytes of SHA-256, hex). `lut::cached` keeps
at most 8 parsed files, re-reading a file when its size or modification time changes.
`lut::read` uses `base::io::read_limited` (regular files only).

## In the filter graph (ADR-110)

`lut_chain` returns, with a leading comma, `format=gbrp,lut3d=file=<name>:interp=tetrahedral[,lut3d=...],format=<pix_fmt>`
(empty without a LUT, so projects without a LUT give the byte-identical 1.5.0 argv: the golden tests
are unchanged). The whole video's table comes first, then the clip's. The cube text goes into the
private Work folder as `lut-whole.cube` / `lut-clip-<segment>.cube` through `Graph::command_files`
(the same path as the camera command files), so no file name or text of the user is in a filter.
Where the filter sits:

- flat clip without a frame effect: after the scale and before the `pad`, so the black bars stay black;
- clip with a frame effect (Change aspect, Scale, Move): on the fitted picture before the frame is composed,
  so a solid colour background keeps its colour; a blurred background, which is made from the picture,
  carries the LUT colours (deviation from ADR-110, which wanted it untouched; assumption: the blur is part of
  the picture);
- 360 camera view: after the camera cut, or before the texts fixed to the sphere when there are any (they keep
  their colours); "360 video" export (`Sphere360`) applies it to the equirectangular picture;
- subtitles and text items are burned after the concat and are never coloured.

Any LUT forces a re-encode (`plan_export_choice`). The hardware export uses the same graph.

## Missing or changed file (REQ-136)

- Opening a project: a missing LUT file does not stop it; the name is added to the "could not be found"
  notice and the LUT is ignored in the preview (`Source::Preview`).
- Export (`Source::Export`): `segments()` fails with `RenderError::Lut(name, why)` before anything is
  written, naming the file; a file that became invalid is refused the same way.
- Choosing a file validates it first (`app::lut::set_lut_file`); a bad file gives the parser's message and
  changes nothing.
- A changed file changes `FrameKey.luts`, so the preview picture is rendered again once.

## Undo and project format 8

Choose, change and remove are one undo step each ("Apply LUT", "Change LUT", "Remove LUT"). Split gives both
halves the LUT; merge needs the same LUT on both ("The clips have different colour effects"); removing a clip
drops its entry; paste carries it. The project file always says `schema_version` 8; the two keys are skipped
when empty. 1.5.x refuses a schema-8 file with its newer-version message (checked by a helper that mimics 1.5,
and by hand with the real 1.5.0 build: not a unit test). Damaged entries (empty, over 4096 characters, with a NUL,
unknown clip, duplicate) are removed with a note; a path that is not text is refused as a damaged file.

## Verification of `lut3d`

Checked with ffmpeg 6.1.1 (the development machine): `format=gbrp,lut3d=file=<relative cube>:interp=tetrahedral,format=yuv420p`
gives exact channel swaps (`tests/ffmpeg_integration/v16.rs`). **The bundled 9.0.2 build was downloaded (its SHA-256
matches `packaging/ffmpeg.lock`) but not run: the sandbox refused to execute a downloaded binary, so `lut3d` on 9.x is
unverified here.** `scripts/package.sh` now lists `lut3d` in the smoke test of the bundled ffmpeg, so a build without it
is refused at packaging time; run the v16 integration tests against the 9.x build before the release.

## Tests

`src/lut.rs` (parser table, property test with random bytes, 1D to 3D, caps), `src/timeline.rs`, `src/project.rs`
(v8 round trip, v7 unchanged, damaged and missing), `src/render/` (graph order, no user name in the graph,
re-encode, preview ignores a missing file, frame key), `src/app/lut_tests.rs`, `src/ui/lut_ux_tests.rs`, and the real
ffmpeg tests `tests/ffmpeg_integration/v16.rs` (clip 2 of 3 only, preview versus export within 4 levels, whole then
clip, solid background untouched, missing file writes nothing, 360 export).

## Known limits

No `DOMAIN_MIN/MAX` other than 0..1, no strength, no LUT library, no `.3dl`/`.csp`. The speed of a 33^3 or 65^3 table
on weak computers is not measured.
