#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Rebuilds the alignment fixture (development only; the results are committed):
#   speech.mkv      a small video whose sound is synthetic speech, with a matching
#                   English subtitle track (one word misspelt on purpose)
#   reference.json  where every word starts in the sound (seconds), known exactly
#                   because each word is synthesised on its own and placed by this script
#
# espeak-ng is used only here, never when the tests run (CI installs it with the other
# packages, see build.yml; the version used for the committed files was not recorded).
# The tests check the committed files by SHA-256: update the sums in
# tests/ffmpeg_integration/v12.rs when you regenerate them.
# Needs: espeak-ng, ffmpeg, ffprobe (with libopus and libx264 or mpeg4).
# Usage: tests/fixtures/align/make.sh
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# One phrase per subtitle line. "quik" in the track is the misspelling of "quick".
phrases=(
  "hello world this is a test"
  "the quick brown fox jumps over the lazy dog"
  "subtitles follow the speech word by word"
  "thank you for watching this video"
)
track=(
  "Hello world, this is a test."
  "The quik brown fox jumps over the lazy dog."
  "Subtitles follow the speech, word by word."
  "Thank you for watching this video!"
)

rate=22050
word_gap="${WORD_GAP:-0.15}"     # silence between two words of a phrase
phrase_gap=1.20   # silence between two phrases
lead=0.50         # silence before the first word

silence() { # seconds, file
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "anullsrc=r=$rate:cl=mono" \
    -t "$1" -c:a pcm_s16le "$2"
}
duration() {
  ffprobe -v error -show_entries format=duration -of csv=p=0 "$1"
}
stamp() { # seconds -> HH:MM:SS,mmm
  awk -v t="$1" 'BEGIN { ms = int(t * 1000 + 0.5); printf "%02d:%02d:%02d,%03d",
    ms / 3600000, (ms / 60000) % 60, (ms / 1000) % 60, ms % 1000 }'
}

silence "$word_gap" "$work/gap.wav"
silence "$phrase_gap" "$work/pause.wav"
silence "$lead" "$work/lead.wav"

list="$work/list.txt"
printf "file '%s'\n" "$work/lead.wav" > "$list"
t="$lead"
n=0
json="["
: > "$work/subs.srt"
for p in "${!phrases[@]}"; do
  first=""
  for word in ${phrases[$p]}; do
    n=$((n + 1))
    raw="$work/raw$n.wav"
    w="$work/w$n.wav"
    espeak-ng -v en-us -s "${SPEED:-135}" -w "$raw" "$word"
    # same sample format for every piece, leading/trailing silence of the voice removed
    ffmpeg -hide_banner -loglevel error -y -i "$raw" -af \
      "silenceremove=start_periods=1:start_threshold=-50dB,areverse,silenceremove=start_periods=1:start_threshold=-50dB,areverse" \
      -ar "$rate" -ac 1 -c:a pcm_s16le "$w"
    d="$(duration "$w")"
    [ -n "$first" ] || first="$t"
    [ "$json" = "[" ] || json="$json,"
    json="$json
  {\"word\": \"$word\", \"start\": $(awk -v t="$t" 'BEGIN { printf "%.3f", t }')}"
    printf "file '%s'\n" "$w" >> "$list"
    t="$(awk -v a="$t" -v b="$d" 'BEGIN { printf "%.6f", a + b }')"
    last="$t"
    printf "file '%s'\n" "$work/gap.wav" >> "$list"
    t="$(awk -v a="$t" -v b="$word_gap" 'BEGIN { printf "%.6f", a + b }')"
  done
  from="$(awk -v a="$first" 'BEGIN { printf "%.3f", a - 0.10 }')"
  to="$(awk -v a="$last" 'BEGIN { printf "%.3f", a + 0.25 }')"
  printf '%d\n%s --> %s\n%s\n\n' "$((p + 1))" "$(stamp "$from")" "$(stamp "$to")" \
    "${track[$p]}" >> "$work/subs.srt"
  printf "file '%s'\n" "$work/pause.wav" >> "$list"
  t="$(awk -v a="$t" -v b="$phrase_gap" 'BEGIN { printf "%.6f", a + b }')"
done
json="$json
]"

ffmpeg -hide_banner -loglevel error -y -f concat -safe 0 -i "$list" -c:a pcm_s16le "$work/speech.wav"
total="$(duration "$work/speech.wav")"

venc=libx264
ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libx264 || venc=mpeg4
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "color=c=black:s=160x90:r=5:d=$total" \
  -i "$work/speech.wav" -i "$work/subs.srt" \
  -map 0:v -map 1:a -map 2:s \
  -c:v "$venc" -pix_fmt yuv420p -g 25 -b:v 8k \
  -c:a libopus -b:a 24k -ac 1 \
  -c:s srt -metadata:s:s:0 language=eng -metadata:s:s:0 title="English" \
  -shortest "$here/speech.mkv"
printf '%s\n' "$json" > "$here/reference.json"
cp "$work/subs.srt" "$here/track.srt"
ls -l "$here/speech.mkv" "$here/reference.json"
