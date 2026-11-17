#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Builds the installers of one target from an already built release executable
# (cargo build --release --locked) and the pinned ffmpeg (scripts/fetch-ffmpeg.sh).
#   scripts/package.sh linux-x86_64     -> videolore-${VERSION}-linux-x86_64.AppImage
#                                          videolore-${VERSION}-linux-x86_64.tar.gz
#   scripts/package.sh windows-x86_64   -> videolore-${VERSION}-windows-x86_64-setup.exe
#   scripts/package.sh macos-aarch64    -> videolore-${VERSION}-macos-aarch64.dmg
#   scripts/package.sh sources          -> videolore-${VERSION}-third-party-source.tar.xz
#   scripts/package.sh windows-x86_64|macos-aarch64 --config-only -> only dist/stage-*/Packager.toml
# Output goes to dist/. Tools: cargo-packager (Windows, macOS; version in
# packaging/tools.lock), appimagetool + type2 runtime (Linux, pinned by SHA-256), curl, git.
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:?usage: package.sh linux-x86_64|windows-x86_64|macos-aarch64|sources}"
VERSION=$(scripts/version.sh get)
dist=dist
stage="$dist/stage-$target"
ffdir="$dist/ffmpeg-$target"
mkdir -p "$dist"

# Value of KEY in section [SECTION] of a lock file: lockval FILE SECTION KEY
lockval() {
    awk -v sec="[$2]" -v key="$3" '
        /^[[:space:]]*#/ { next }
        /^\[/ { insec = ($0 == sec); next }
        insec { split($0, kv, "="); k = kv[1]; gsub(/[[:space:]]/, "", k)
                if (k == key) { v = substr($0, index($0, "=") + 1); gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); print v; exit } }
    ' "$1"
}

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# fetch_tool NAME FILE: downloads a pinned tool from packaging/tools.lock and checks it.
fetch_tool() {
    local url sha
    url=$(lockval packaging/tools.lock "$1" url)
    sha=$(lockval packaging/tools.lock "$1" sha256)
    curl -fsSL --retry 3 -o "$2" "$url"
    [[ "$(sha256 "$2")" == "$sha" ]] || { echo "checksum mismatch for $1" >&2; exit 1; }
}

# Video hardware encoders ("Export: Faster") each bundled ffmpeg must have.
hw_encoders() {
    case "$1" in
    linux-x86_64) echo h264_nvenc hevc_nvenc h264_qsv hevc_qsv h264_vaapi hevc_vaapi ;;
    windows-x86_64) echo h264_nvenc hevc_nvenc h264_qsv hevc_qsv h264_amf hevc_amf ;;
    macos-aarch64) echo h264_videotoolbox hevc_videotoolbox ;;
    esac
}

# The bundled ffmpeg must have everything videolore needs.
smoke_test() {
    local ff="$1"
    "$ff" -hide_banner -version | head -1
    local enc filters
    enc=$("$ff" -hide_banner -encoders)
    filters=$("$ff" -hide_banner -filters)
    for e in libx264 libx265 libvpx-vp9 libopus libmp3lame $(hw_encoders "$target"); do
        grep -qw "$e" <<<"$enc" || { echo "bundled ffmpeg lacks encoder $e" >&2; exit 1; }
    done
    for f in subtitles amix alimiter psnr afftdn; do
        grep -qw "$f" <<<"$filters" || { echo "bundled ffmpeg lacks filter $f" >&2; exit 1; }
    done
    echo "smoke test passed: $ff"
}

# Notices and the subtitle font licence shipped with every package.
stage_notices() {
    mkdir -p "$1/notices" "$1/fonts"
    cp packaging/notices/third-party.txt packaging/notices/rust-crates.txt LICENSE "$1/notices/"
    cp packaging/ffmpeg.lock "$1/notices/ffmpeg-builds.txt"
    cp assets/fonts/OFL.txt assets/fonts/SOURCE.txt "$1/fonts/"
}

# Absolute path in the form the native tools understand (D:/a/... under Git Bash on
# Windows), safe inside a TOML basic string.
native_path() {
    local p="$1"
    [[ "$p" == /* ]] || p="$PWD/$p"
    if command -v cygpath >/dev/null 2>&1; then cygpath -m "$p"; else printf '%s\n' "$p"; fi
}

# Writes $stage/Packager.toml for cargo-packager (Windows NSIS, macOS app).
# cargo-packager reads every relative path from the directory of the config file, so
# every path written here is absolute. Keys and values follow cargo-packager 0.11.8
# (packaging/tools.lock): enum values are camelCase ("editor", "currentUser"), the NSIS
# install mode key is "installMode" (it has no "install-mode" alias), and "name" must be
# set: without it cargo-packager tries to enter the config file as if it were a folder
# to look for a Cargo.toml, and stops with "Not a directory".
packager_config() {
    local root res formats dll
    root=$(native_path "$PWD")
    res=$(native_path "$stage/res")
    local resources='"'"$res/notices"'", "'"$res/fonts"'"'
    if [[ "$target" == windows-x86_64 ]]; then
        # the DLLs of the shared ffmpeg build go next to ffmpeg.exe (the install folder)
        for dll in "$ffdir"/*.dll; do
            [[ -e "$dll" ]] || continue
            resources+=', { src = "'"$(native_path "$dll")"'", target = "'"$(basename "$dll")"'" }'
        done
        formats='["nsis"]'
    else
        formats='["app"]'
    fi
    mkdir -p "$stage"
    cat >"$stage/Packager.toml" <<TOML
name = "videolore"
product-name = "videolore"
identifier = "io.github.andrealorenzani.videolore"
version = "${VERSION}"
description = "Non-destructive video editor for short social videos"
authors = ["Andrea Lorenzani"]
publisher = "Andrea Lorenzani"
license-file = "${root}/packaging/notices/third-party.txt"
out-dir = "$(native_path "$stage/out")"
binaries-dir = "$(native_path "$stage/bin")"
binaries = [
    { path = "videolore", main = true },
    { path = "ffmpeg", main = false },
    { path = "ffprobe", main = false },
]
icons = ["${root}/assets/icon/icon.png", "${root}/assets/icon/icon.ico", "${root}/assets/icon/icon.icns"]
resources = [${resources}]
formats = ${formats}
file-associations = [
    { extensions = ["vlproj"], name = "videolore project", description = "videolore project", mime-type = "application/x-videolore-project", role = "editor" },
]

[nsis]
installMode = "currentUser"

[macos]
minimum-system-version = "11.0"
TOML
}

# package.sh TARGET --config-only: only writes $stage/Packager.toml (no ffmpeg, no
# build), so the configuration can be checked on any machine.
if [[ "${2:-}" == --config-only ]]; then
    case "$target" in
    windows-x86_64 | macos-aarch64) packager_config; echo "$stage/Packager.toml"; exit 0 ;;
    *) echo "--config-only is for windows-x86_64 and macos-aarch64" >&2; exit 2 ;;
    esac
fi

[[ -d "$ffdir" || "$target" == sources ]] || scripts/fetch-ffmpeg.sh "$target" "$ffdir"

case "$target" in
linux-x86_64)
    exe=target/release/videolore
    smoke_test "$ffdir/ffmpeg"
    # The video hardware libraries (libva, libcuda, libvpl) come from the system at run
    # time: the bundled ffmpeg must be dynamically linked against glibc to load them.
    file -L "$ffdir/ffmpeg" | grep -q "dynamically linked" ||
        { echo "the bundled ffmpeg is not dynamically linked: video hardware cannot load" >&2; exit 1; }
    # The packages must run on current Ubuntu LTS and Fedora: refuse an executable that
    # needs a newer glibc than Ubuntu 24.04's 2.39.
    max_glibc="2.39"
    need=$(objdump -T "$exe" | grep -oE 'GLIBC_[0-9]+\.[0-9]+' | sed 's/GLIBC_//' | sort -V | tail -1)
    if [[ "$(printf '%s\n%s\n' "$need" "$max_glibc" | sort -V | tail -1)" != "$max_glibc" ]]; then
        echo "videolore needs glibc $need, newer than $max_glibc" >&2; exit 1
    fi
    echo "glibc needed: $need (limit $max_glibc)"
    # --- AppImage: hand-built AppDir + appimagetool with the pinned static runtime
    appdir="$stage/AppDir"
    rm -rf "$stage" && mkdir -p "$appdir/usr/bin" "$appdir/usr/share/videolore"
    install -m 0755 "$exe" "$appdir/usr/bin/videolore"
    install -m 0755 "$ffdir/ffmpeg" "$ffdir/ffprobe" "$appdir/usr/bin/"
    stage_notices "$appdir/usr/share/videolore"
    cp assets/icon/icon-256.png "$appdir/videolore.png"
    cat >"$appdir/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/videolore" "$@"
APPRUN
    chmod 0755 "$appdir/AppRun"
    cat >"$appdir/videolore.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=videolore
Comment=Edit short videos for social media
Exec=videolore %f
Icon=videolore
Terminal=false
Categories=AudioVideo;Video;AudioVideoEditing;
MimeType=application/x-videolore-project;
DESKTOP
    fetch_tool appimagetool "$stage/appimagetool"
    fetch_tool appimage-runtime "$stage/runtime"
    chmod 0755 "$stage/appimagetool"
    ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$stage/appimagetool" --no-appstream \
        --runtime-file "$stage/runtime" "$appdir" "$dist/videolore-${VERSION}-linux-x86_64.AppImage"
    chmod 0755 "$dist/videolore-${VERSION}-linux-x86_64.AppImage"
    # --- tar.gz: a folder that runs by double-clicking `videolore`
    top="videolore-${VERSION}-linux-x86_64"
    folder="$stage/$top"
    mkdir -p "$folder"
    install -m 0755 "$exe" "$ffdir/ffmpeg" "$ffdir/ffprobe" "$folder/"
    stage_notices "$folder"
    cp assets/icon/icon-256.png "$folder/videolore.png"
    : >"$folder/.videolore-bundle"
    cat >"$folder/README.txt" <<README
videolore ${VERSION} for Linux (x86_64)

Double-click "videolore" in this folder to start it. The first start adds videolore to
your application menu and lets you open .vlproj projects with a double-click.

If double-clicking does not start it, right-click "videolore" and choose "Run", or run
./videolore once from a terminal in this folder.

To uninstall: use "Remove from application menu" in videolore's About screen, then delete
this folder. Your projects are never touched.
README
    tar -C "$stage" --owner=0 --group=0 -czf "$dist/videolore-${VERSION}-linux-x86_64.tar.gz" "$top"
    ;;
windows-x86_64 | macos-aarch64)
    if [[ "$target" == windows-x86_64 ]]; then ext=.exe; else ext=""; fi
    smoke_test "$ffdir/ffmpeg$ext"
    rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/res"
    cp "target/release/videolore$ext" "$ffdir/ffmpeg$ext" "$ffdir/ffprobe$ext" "$stage/bin/"
    stage_notices "$stage/res"
    if [[ "$target" == windows-x86_64 ]] && ! compgen -G "$ffdir/*.dll" >/dev/null; then
        echo "the Windows ffmpeg build has no DLLs in $ffdir: it would not start" >&2; exit 1
    fi
    packager_config
    cargo packager --release --config "$stage/Packager.toml"
    if [[ "$target" == windows-x86_64 ]]; then
        setup=$(find "$stage/out" -name '*-setup.exe' | head -1)
        [[ -n "$setup" ]] || { echo "cargo-packager made no setup.exe in $stage/out" >&2; exit 1; }
        cp "$setup" "$dist/videolore-${VERSION}-windows-x86_64-setup.exe"
    else
        app=$(find "$stage/out" -maxdepth 2 -name '*.app' -type d | head -1)
        [[ -n "$app" ]] || { echo "cargo-packager made no .app in $stage/out" >&2; exit 1; }
        # ad-hoc re-seal so the nested ffmpeg/ffprobe are covered by the bundle signature
        codesign --force --deep --sign - "$app"
        codesign --verify --deep "$app"
        smoke_test "$app/Contents/MacOS/ffmpeg"
        dmgroot="$stage/dmg"
        mkdir -p "$dmgroot"
        cp -R "$app" "$dmgroot/videolore.app"
        ln -s /Applications "$dmgroot/Applications"
        # hdiutil sometimes fails with "Resource busy" on CI machines: retry a few times
        for attempt in 1 2 3 4 5; do
            hdiutil create -volname videolore -srcfolder "$dmgroot" -ov -format UDZO \
                "$dist/videolore-${VERSION}-macos-aarch64.dmg" && break
            [[ $attempt -lt 5 ]] || { echo "hdiutil failed $attempt times" >&2; exit 1; }
            sleep 5
        done
    fi
    ;;
sources)
    # Corresponding source of the bundled third-party programs (GPL compliance).
    src="$dist/stage-sources/videolore-${VERSION}-third-party-source"
    rm -rf "$dist/stage-sources" && mkdir -p "$src"
    for t in linux-x86_64 windows-x86_64 macos-aarch64; do
        url=$(lockval packaging/ffmpeg.lock "$t" source)
        file="$src/ffmpeg-source-$(basename "$url")"
        if [[ ! -f "$file" ]]; then
            curl -fsSL --retry 3 -o "$file" "$url"
            want=$(lockval packaging/ffmpeg.lock "$t" source_sha256)
            if [[ -n "$want" && "$(sha256 "$file")" != "$want" ]]; then
                echo "checksum mismatch for $url" >&2
                exit 1
            fi
            if [[ "$url" == https://ffmpeg.org/* ]]; then
                curl -fsSL --retry 3 -o "$file.asc" "$url.asc" || true
            fi
        fi
        repo=$(lockval packaging/ffmpeg.lock "$t" build_scripts)
        commit=$(lockval packaging/ffmpeg.lock "$t" build_scripts_commit)
        out="$src/build-scripts-$(basename "$repo")-$commit.tar.gz"
        if [[ ! -f "$out" ]]; then
            git clone --quiet --filter=blob:none "$repo" "$dist/stage-sources/repo-$commit"
            git -C "$dist/stage-sources/repo-$commit" archive --format=tar.gz -o "$PWD/$out" "$commit"
        fi
    done
    cp packaging/ffmpeg.lock "$src/ffmpeg-builds.txt"
    cp packaging/notices/third-party.txt "$src/"
    cp assets/fonts/OFL.txt assets/fonts/SOURCE.txt "$src/"
    cat >"$src/README.txt" <<README
Third-party source code for videolore ${VERSION}

- ffmpeg-source-*: the FFmpeg source code of the bundled ffmpeg and ffprobe builds
  (see ffmpeg-builds.txt for which build each platform ships).
- build-scripts-*: the scripts that produced those builds, at the exact commit used.
  They name and pin the version of every library compiled into ffmpeg (x264, x265,
  libass, ...), and download their sources.
- SOURCE.txt / OFL.txt: the subtitle font (Noto Sans Bold) and its licence.
- third-party.txt: the notices shown in videolore, including the written offer.
README
    tar -C "$dist/stage-sources" -cJf "$dist/videolore-${VERSION}-third-party-source.tar.xz" \
        "videolore-${VERSION}-third-party-source"
    ;;
*)
    echo "unknown target $target" >&2
    exit 2
    ;;
esac
ls -l "$dist"/videolore-"${VERSION}"-*
