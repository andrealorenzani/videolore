#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Builds the installers of one product (the editor by default, videolore player or
# videolore screen spy) for one target from an already built release executable (cargo build
# --release --locked, with -p videolore-player or -p videolore-screen-spy) and the pinned
# ffmpeg (scripts/fetch-ffmpeg.sh).
#   scripts/package.sh linux-x86_64 [player]    -> <name>-${VERSION}-linux-x86_64.AppImage
#                                                  <name>-${VERSION}-linux-x86_64.tar.gz
#   scripts/package.sh windows-x86_64 [player]  -> <name>-${VERSION}-windows-x86_64-setup.exe
#   scripts/package.sh macos-aarch64 [player]   -> <name>-${VERSION}-macos-aarch64.dmg
#   scripts/package.sh sources                  -> videolore-${VERSION}-third-party-source.tar.xz
#   scripts/package.sh windows-x86_64|macos-aarch64 [player] --config-only
#                                               -> only dist/stage-*/Packager.toml
#   scripts/package.sh linux-x86_64 [player] --stub
#                                               -> an AppImage of stub programs in dist/stub/,
#                                                  to check the AppDir and appimagetool (CI)
#   scripts/package.sh tool NAME                -> downloads the pinned tool NAME of
#                                                  packaging/tools.lock and checks its SHA-256
#                                                  (VIDEOLORE_TOOLS_LOCK names another lock file)
# <name> is videolore (editor), videolore-player or videolore-screen-spy. Output goes to dist/. Tools:
# cargo-packager (Windows, macOS; version in packaging/tools.lock), appimagetool + type2
# runtime (Linux, pinned by SHA-256), curl, git.
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:?usage: package.sh linux-x86_64|windows-x86_64|macos-aarch64|sources|tool [editor|player|screen-spy] [--config-only]}"
shift
toolslock="${VIDEOLORE_TOOLS_LOCK:-packaging/tools.lock}"
product=editor
config_only=0
stub=0
for a in "$@"; do
    case "$a" in
    editor | player | screen-spy) product="$a" ;;
    --config-only) config_only=1 ;;
    --stub) stub=1 ;;
    *) if [[ "$target" == tool ]]; then product="$a"; else echo "unknown argument $a" >&2; exit 2; fi ;;
    esac
done
VERSION=$(scripts/version.sh get)
dist=dist
# stub packages are only a check of the packaging: never mixed with real output
[[ $stub -eq 0 ]] || dist=dist/stub
# One product, one set of names (the installers differ mainly by these).
if [[ "$product" == player ]]; then
    pname=videolore-player
    pdisplay="videolore player"
    pident=io.github.andrealorenzani.videolore-player
    pdesc="Simple video player with a 360 view and subtitles"
    pcrates=rust-crates-player.txt
    pthird=third-party-player.txt
    stage="$dist/stage-$target-player"
elif [[ "$product" == screen-spy ]]; then
    pname=videolore-screen-spy
    pdisplay="videolore screen spy"
    pident=io.github.andrealorenzani.videolore-screen-spy
    pdesc="Records the screen with the computer sound and the microphone"
    pcrates=rust-crates-screen-spy.txt
    pthird=third-party-screen-spy.txt
    stage="$dist/stage-$target-screen-spy"
else
    pname=videolore
    pdisplay=videolore
    pident=io.github.andrealorenzani.videolore
    pdesc="Non-destructive video editor for short social videos"
    pcrates=rust-crates.txt
    pthird=third-party.txt
    stage="$dist/stage-$target"
fi
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
    url=$(lockval "$toolslock" "$1" url)
    sha=$(lockval "$toolslock" "$1" sha256)
    [[ -n "$url" && -n "$sha" ]] || { echo "no pinned tool $1 in $toolslock" >&2; exit 1; }
    curl -fsSL --retry 3 -o "$2" "$url"
    [[ "$(sha256 "$2")" == "$sha" ]] || { echo "checksum mismatch for $1" >&2; exit 1; }
}

# package.sh tool NAME: the pinned tool is downloaded and checked, nothing else (CI, tests).
if [[ "$target" == tool ]]; then
    name="${product}"
    [[ "$name" != editor ]] || { echo "usage: package.sh tool NAME" >&2; exit 2; }
    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT
    fetch_tool "$name" "$tmp"
    echo "$name is the pinned file"
    exit 0
fi

# Video hardware encoders ("Export: Faster") each bundled ffmpeg must have.
hw_encoders() {
    case "$1" in
    linux-x86_64) echo h264_nvenc hevc_nvenc h264_qsv hevc_qsv h264_vaapi hevc_vaapi ;;
    windows-x86_64) echo h264_nvenc hevc_nvenc h264_qsv hevc_qsv h264_amf hevc_amf ;;
    macos-aarch64) echo h264_videotoolbox hevc_videotoolbox ;;
    esac
}

# What the screen recorder captures with on each system.
capture_devices() {
    case "$1" in
    linux-x86_64) echo x11grab pulse ;;
    windows-x86_64) echo gdigrab ;;
    macos-aarch64) echo avfoundation ;;
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
    for f in subtitles amix alimiter psnr afftdn lut3d; do
        grep -qw "$f" <<<"$filters" || { echo "bundled ffmpeg lacks filter $f" >&2; exit 1; }
    done
    if [[ "$product" == screen-spy ]]; then
        local devices
        devices=$("$ff" -hide_banner -devices)
        for d in $(capture_devices "$target"); do
            grep -qw "$d" <<<"$devices" || { echo "bundled ffmpeg lacks capture device $d" >&2; exit 1; }
        done
        for e in aac pcm_s16le; do
            grep -qw "$e" <<<"$enc" || { echo "bundled ffmpeg lacks encoder $e" >&2; exit 1; }
        done
    fi
    echo "smoke test passed: $ff"
}

# Notices and the subtitle font licence shipped with every package.
stage_notices() {
    mkdir -p "$1/notices" "$1/icons"
    # each program names its own components (the player has no speech engine)
    cp "packaging/notices/$pthird" "$1/notices/third-party.txt"
    cp LICENSE "$1/notices/"
    cp "packaging/notices/$pcrates" "$1/notices/"
    cp packaging/ffmpeg.lock "$1/notices/ffmpeg-builds.txt"
    cp assets/icons/LICENSE-Phosphor.txt assets/icons/SOURCE.txt "$1/icons/"
    # the screen recorder draws no subtitles: it carries no subtitle font
    [[ "$product" != screen-spy ]] || return 0
    mkdir -p "$1/fonts"
    cp assets/fonts/OFL.txt assets/fonts/SOURCE.txt "$1/fonts/"
    # the subtitle fonts are compiled into the program: report what they add (REQ-187: at
    # most 25 MB for the scripts added since 2.0.3; the Latin face is the 2.0.3 baseline)
    local added=0 f
    for f in assets/fonts/NotoSansArabic-Bold.ttf assets/fonts/NotoSansDevanagari-Bold.ttf; do
        added=$((added + $(wc -c <"$f")))
    done
    echo "subtitle fonts added in 2.0.4 (Arabic, Devanagari): $added bytes"
    [ "$added" -le $((25 * 1024 * 1024)) ] || { echo "the added fonts exceed 25 MB" >&2; exit 1; }
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
    local root res formats dll licence
    root=$(native_path "$PWD")
    res=$(native_path "$stage/res")
    local resources='"'"$res/notices"'", "'"$res/icons"'"'
    [[ "$product" == screen-spy ]] || resources+=', "'"$res/fonts"'"'
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
    # the editor opens projects; the player is offered for the six video types it plays
    # (REQ-248): macOS lists it under "Open With" with the Viewer role, which never makes
    # it the default by itself
    local assoc=""
    # the screen recorder opens no files: no association; macOS shows its two sound
    # permission texts
    local plist=""
    if [[ "$product" == screen-spy ]]; then
        plist='info-plist-path = "'"$(native_path "$PWD/packaging/macos/screen-spy-info.plist")"'"'
    elif [[ "$product" == editor ]]; then
        assoc='file-associations = [
    { extensions = ["vlproj"], name = "videolore project", description = "videolore project", mime-type = "application/x-videolore-project", role = "editor" },
]'
    else
        assoc='file-associations = [
    { extensions = ["mp4", "m4v"], name = "MPEG-4 video", description = "MPEG-4 video", mime-type = "video/mp4", role = "viewer" },
    { extensions = ["mov"], name = "QuickTime movie", description = "QuickTime movie", mime-type = "video/quicktime", role = "viewer" },
    { extensions = ["mkv"], name = "Matroska video", description = "Matroska video", mime-type = "video/x-matroska", role = "viewer" },
    { extensions = ["webm"], name = "WebM video", description = "WebM video", mime-type = "video/webm", role = "viewer" },
    { extensions = ["avi"], name = "AVI video", description = "AVI video", mime-type = "video/x-msvideo", role = "viewer" },
]'
    fi
    # the player has its own installer template: the video types are "Open with" entries only
    local nsis_template=""
    if [[ "$product" == player && "$target" == windows-x86_64 ]]; then
        nsis_template='template = "'"$(native_path "$PWD/packaging/windows-player.nsi")"'"'
    fi
    mkdir -p "$stage"
    # The licence page of the Windows installer reads the file as UTF-8 only when it starts
    # with a byte-order mark: the notices have non-ASCII text (REQ-229). The copy in the
    # repository stays plain (the program shows it as it is).
    licence="$stage/third-party-licence.txt"
    { printf '\357\273\277'; cat "packaging/notices/$pthird"; } >"$licence"
    cat >"$stage/Packager.toml" <<TOML
name = "${pname}"
product-name = "${pdisplay}"
identifier = "${pident}"
version = "${VERSION}"
description = "${pdesc}"
authors = ["Andrea Lorenzani"]
publisher = "Andrea Lorenzani"
license-file = "$(native_path "$licence")"
out-dir = "$(native_path "$stage/out")"
binaries-dir = "$(native_path "$stage/bin")"
binaries = [
    { path = "${pname}", main = true },
    { path = "ffmpeg", main = false },
    { path = "ffprobe", main = false },
]
icons = ["${root}/assets/icon/icon.png", "${root}/assets/icon/icon.ico", "${root}/assets/icon/icon.icns"]
resources = [${resources}]
formats = ${formats}
${assoc}

[nsis]
installMode = "currentUser"
${nsis_template}

[macos]
minimum-system-version = "11.0"
${plist}
TOML
}

# package.sh TARGET --config-only: only writes $stage/Packager.toml (no ffmpeg, no
# build), so the configuration can be checked on any machine.
if [[ $config_only -eq 1 ]]; then
    case "$target" in
    windows-x86_64 | macos-aarch64) packager_config; echo "$stage/Packager.toml"; exit 0 ;;
    *) echo "--config-only is for windows-x86_64 and macos-aarch64" >&2; exit 2 ;;
    esac
fi

if [[ $stub -eq 1 ]]; then
    [[ "$target" == linux-x86_64 ]] || { echo "--stub is for linux-x86_64" >&2; exit 2; }
    # stub programs stand for the executable and the pinned ffmpeg (nothing is downloaded
    # but appimagetool and its runtime, which are pinned and checked)
    rm -rf "$ffdir" && mkdir -p "$ffdir" "$dist/stub-bin"
    for f in "$ffdir/ffmpeg" "$ffdir/ffprobe" "$dist/stub-bin/$pname"; do
        printf '#!/bin/sh\nexit 0\n' >"$f"
    done
elif [[ ! -d "$ffdir" && "$target" != sources ]]; then
    scripts/fetch-ffmpeg.sh "$target" "$ffdir"
fi

case "$target" in
linux-x86_64)
    exe="target/release/$pname"
    [[ $stub -eq 0 ]] || exe="$dist/stub-bin/$pname"
    if [[ $stub -eq 0 ]]; then
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
        echo "$pname needs glibc $need, newer than $max_glibc" >&2; exit 1
    fi
    echo "glibc needed: $need (limit $max_glibc)"
    fi
    # --- AppImage: hand-built AppDir + appimagetool with the pinned static runtime
    appdir="$stage/AppDir"
    rm -rf "$stage" && mkdir -p "$appdir/usr/bin" "$appdir/usr/share/$pname"
    install -m 0755 "$exe" "$appdir/usr/bin/$pname"
    install -m 0755 "$ffdir/ffmpeg" "$ffdir/ffprobe" "$appdir/usr/bin/"
    stage_notices "$appdir/usr/share/$pname"
    # both products use the editor's icon artwork (they differ by name and identifier)
    cp assets/icon/icon-256.png "$appdir/$pname.png"
    cat >"$appdir/AppRun" <<APPRUN
#!/bin/sh
HERE="\$(dirname "\$(readlink -f "\$0")")"
exec "\$HERE/usr/bin/$pname" "\$@"
APPRUN
    chmod 0755 "$appdir/AppRun"
    if [[ "$product" == screen-spy ]]; then
        cp packaging/linux/screen-spy.desktop "$appdir/$pname.desktop"
    elif [[ "$product" == player ]]; then
        cat >"$appdir/$pname.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$pdisplay
Comment=Watch a video, with its 360 view and subtitles
Exec=$pname %f
Icon=$pname
Terminal=false
Categories=AudioVideo;Player;Video;
MimeType=video/mp4;video/quicktime;video/x-matroska;video/webm;video/x-m4v;video/x-msvideo;
DESKTOP
    else
        cat >"$appdir/$pname.desktop" <<DESKTOP
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
    fi
    fetch_tool appimagetool "$stage/appimagetool"
    fetch_tool appimage-runtime "$stage/runtime"
    chmod 0755 "$stage/appimagetool"
    ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$stage/appimagetool" --no-appstream \
        --runtime-file "$stage/runtime" "$appdir" "$dist/${pname}-${VERSION}-linux-x86_64.AppImage"
    chmod 0755 "$dist/${pname}-${VERSION}-linux-x86_64.AppImage"
    # --- tar.gz (a stub run makes one too, to check its content): a folder that runs by double-clicking `$pname`
    top="${pname}-${VERSION}-linux-x86_64"
    folder="$stage/$top"
    mkdir -p "$folder"
    install -m 0755 "$exe" "$ffdir/ffmpeg" "$ffdir/ffprobe" "$folder/"
    stage_notices "$folder"
    cp assets/icon/icon-256.png "$folder/$pname.png"
    # A launcher and a desktop entry: file managers that do not start an extracted program
    # on double-click can start it from the launcher or from the entry (REQ-231).
    sed "s/@NAME@/$pname/g" packaging/linux/launcher.sh >"$folder/$pname.sh"
    chmod 0755 "$folder/$pname.sh"
    sed -e "s/@NAME@/$pname/g" -e "s/@DISPLAY@/$pdisplay/g" packaging/linux/launcher.desktop >"$folder/$pname.desktop"
    chmod 0755 "$folder/$pname.desktop"
    if [[ "$product" == screen-spy ]]; then
        cat >"$folder/README.txt" <<README
videolore screen spy ${VERSION} for Linux (x86_64)

Double-click "videolore-screen-spy" in this folder to start it, or run
./videolore-screen-spy from a terminal in this folder.

If double-clicking does not start it, right-click "videolore-screen-spy" and choose "Run",
or double-click "videolore-screen-spy.sh" (a launcher) or "videolore-screen-spy.desktop" (a
menu entry for file managers that start those).

It records one screen of an X11 session (or XWayland) with the computer sound and the
microphone. Recording on a Wayland session is not possible yet. To uninstall, delete this
folder.
README
    elif [[ "$product" == player ]]; then
        # the player does not add itself to the application menu: it says how to start
        cat >"$folder/README.txt" <<README
videolore player ${VERSION} for Linux (x86_64)

Double-click "videolore-player" in this folder to start it, or run
./videolore-player some-video.mp4 from a terminal in this folder.

If double-clicking does not start it, right-click "videolore-player" and choose "Run", or
double-click "videolore-player.sh" (a launcher) or "videolore-player.desktop" (a menu entry
for file managers that start those).

videolore player plays your videos one after the other. It never changes your files. To uninstall,
delete this folder.
README
    else
    : >"$folder/.videolore-bundle"
    cat >"$folder/README.txt" <<README
videolore ${VERSION} for Linux (x86_64)

Double-click "videolore" in this folder to start it. The first start adds videolore to
your application menu and lets you open .vlproj projects with a double-click.

If double-clicking does not start it, right-click "videolore" and choose "Run", or run
./videolore once from a terminal in this folder. A launcher ("videolore.sh") and a menu entry
("videolore.desktop") are in the folder too, for file managers that start those instead.

To uninstall: use "Remove from application menu" in videolore's About screen, then delete
this folder. Your projects are never touched.
README
    fi
    tar -C "$stage" --owner=0 --group=0 -czf "$dist/${pname}-${VERSION}-linux-x86_64.tar.gz" "$top"
    ;;
windows-x86_64 | macos-aarch64)
    if [[ "$target" == windows-x86_64 ]]; then ext=.exe; else ext=""; fi
    smoke_test "$ffdir/ffmpeg$ext"
    rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/res"
    cp "target/release/$pname$ext" "$ffdir/ffmpeg$ext" "$ffdir/ffprobe$ext" "$stage/bin/"
    stage_notices "$stage/res"
    if [[ "$target" == windows-x86_64 ]] && ! compgen -G "$ffdir/*.dll" >/dev/null; then
        echo "the Windows ffmpeg build has no DLLs in $ffdir: it would not start" >&2; exit 1
    fi
    packager_config
    if [[ "$target" == windows-x86_64 ]]; then
        # cargo-packager downloads this NSIS toolchain itself; the pinned file is checked
        # first, so a changed file fails the build (REQ-228)
        nsis_zip="$stage/nsis.zip"
        fetch_tool nsis "$nsis_zip"
        rm -f "$nsis_zip"
    fi
    cargo packager --release --config "$stage/Packager.toml"
    if [[ "$target" == windows-x86_64 ]]; then
        setup=$(find "$stage/out" -name '*-setup.exe' | head -1)
        [[ -n "$setup" ]] || { echo "cargo-packager made no setup.exe in $stage/out" >&2; exit 1; }
        cp "$setup" "$dist/${pname}-${VERSION}-windows-x86_64-setup.exe"
    else
        app=$(find "$stage/out" -maxdepth 2 -name '*.app' -type d | head -1)
        [[ -n "$app" ]] || { echo "cargo-packager made no .app in $stage/out" >&2; exit 1; }
        # ad-hoc re-seal so the nested ffmpeg/ffprobe are covered by the bundle signature
        codesign --force --deep --sign - "$app"
        codesign --verify --deep "$app"
        smoke_test "$app/Contents/MacOS/ffmpeg"
        dmgroot="$stage/dmg"
        mkdir -p "$dmgroot"
        cp -R "$app" "$dmgroot/$pdisplay.app"
        ln -s /Applications "$dmgroot/Applications"
        # hdiutil sometimes fails with "Resource busy" on CI machines: retry a few times
        for attempt in 1 2 3 4 5; do
            hdiutil create -volname "$pdisplay" -srcfolder "$dmgroot" -ov -format UDZO \
                "$dist/${pname}-${VERSION}-macos-aarch64.dmg" && break
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
    cp assets/icons/LICENSE-Phosphor.txt "$src/LICENSE-Phosphor-Icons.txt"
    cat >"$src/README.txt" <<README
Third-party source code for videolore ${VERSION}

- ffmpeg-source-*: the FFmpeg source code of the bundled ffmpeg and ffprobe builds
  (see ffmpeg-builds.txt for which build each platform ships).
- build-scripts-*: the scripts that produced those builds, at the exact commit used.
  They name and pin the version of every library compiled into ffmpeg (x264, x265,
  libass, ...), and download their sources.
- SOURCE.txt / OFL.txt: the subtitle font (Noto Sans Bold) and its licence.
- LICENSE-Phosphor-Icons.txt: the licence of the icons of both programs (MIT).
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
ls -l "$dist"/"$pname"-"${VERSION}"-*
