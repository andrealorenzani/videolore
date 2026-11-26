#!/usr/bin/env bash
# Copy the workflow of this project into another directory, as described by manifest.txt.
#   clone.sh <target> [--name N] [--author A] [--repo owner/repo] [--on-conflict overwrite|skip]
# Without --on-conflict nothing is written if a target file already exists (exit 3, list printed).
set -euo pipefail
skill=$(cd "$(dirname "$0")" && pwd)
src=$(cd "$skill/../../.." && pwd)
rel=.claude/skills/clone-workflow
die() { echo "clone: $1" >&2; exit "${2:-1}"; }

target=""; name=""; author=""; repo=""; conflict=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --name|--author|--repo|--on-conflict)
      [[ $# -ge 2 ]] || die "$1 needs a value" 2
      case "$1" in --name) name=$2 ;; --author) author=$2 ;; --repo) repo=$2 ;; *) conflict=$2 ;; esac
      shift 2 ;;
    -*) die "unknown option $1" 2 ;;
    *) [[ -z "$target" ]] || die "one target only" 2; target=$1; shift ;;
  esac
done
[[ -n "$target" ]] || die "usage: clone.sh <target> [--name N] [--author A] [--repo owner/repo] [--on-conflict overwrite|skip]" 2
[[ -z "$conflict" || "$conflict" == overwrite || "$conflict" == skip ]] || die "--on-conflict is overwrite or skip" 2
target="${target/#\~/$HOME}"
target=$(realpath -m -- "$target")
[[ "$target/" != "$src/"* ]] || die "the target is inside the source project" 2

# Plan: one "kind<TAB>destination<TAB>argument" line per file.
plan=()
while read -r kind path arg; do
  case "$kind" in
    ''|'#'*|skip) ;;
    copy)
      [[ -e "$src/$path" ]] || die "manifest: $path does not exist (run check.sh)"
      while IFS= read -r f; do plan+=("copy"$'\t'"${f#"$src/"}"$'\t'); done < <(find "$src/$path" -type f | sort) ;;
    template)
      t="$skill/templates/${path/#./dot-}"
      [[ -f "$t" ]] || die "template missing for $path (run check.sh)"
      plan+=("template"$'\t'"$path"$'\t'"$t") ;;
    empty) plan+=("empty"$'\t'"$path"$'\t'"$arg") ;;
    link) plan+=("link"$'\t'"$path"$'\t'"$arg") ;;
    ignore) ;;
    *) die "manifest: unknown kind $kind" ;;
  esac
done < "$skill/manifest.txt"

existing=()
for p in "${plan[@]}"; do
  IFS=$'\t' read -r _ dest _ <<<"$p"
  if [[ -e "$target/$dest" || -L "$target/$dest" ]]; then existing+=("$dest"); fi
done
if [[ ${#existing[@]} -gt 0 && -z "$conflict" ]]; then
  printf 'already in %s:\n' "$target"; printf '  %s\n' "${existing[@]}"
  die "nothing written; choose --on-conflict overwrite or skip" 3
fi

fill() { # replace the known placeholders in one file
  local f=$1 key value
  for key in PROJECT_NAME AUTHOR REPO_SLUG; do
    case $key in PROJECT_NAME) value=$name ;; AUTHOR) value=$author ;; *) value=$repo ;; esac
    [[ -n "$value" ]] || continue
    value=$(printf '%s' "$value" | sed 's/[\\&|]/\\&/g')
    sed -i "s|{{$key}}|$value|g" "$f"
  done
}

copied=0; templated=0; emptied=0; skipped=(); written_workflow=0
for p in "${plan[@]}"; do
  IFS=$'\t' read -r kind dest arg <<<"$p"
  out="$target/$dest"
  if [[ ( -e "$out" || -L "$out" ) && "$conflict" == skip ]]; then skipped+=("$dest"); continue; fi
  mkdir -p "$(dirname "$out")"
  rm -f -- "$out"
  case $kind in
    copy) cp -p "$src/$dest" "$out"; copied=$((copied + 1)) ;;
    template) cp -p "$arg" "$out"; fill "$out"; templated=$((templated + 1))
      [[ "$dest" != docs/workflow.md ]] || written_workflow=1 ;;
    empty)
      if [[ -n "$arg" ]]; then printf '# %s\n\nNot written yet.\n' "$arg" > "$out"; else : > "$out"; fi
      emptied=$((emptied + 1)) ;;
    link) ln -s "$arg" "$out" ;;
  esac
done
[[ $written_workflow -eq 0 ]] || echo "- $(date '+%Y-%m-%d') — cloned from $src" >> "$target/docs/workflow.md"

# .gitignore lines are merged, never overwritten.
while read -r kind line; do
  [[ "$kind" == ignore ]] || continue
  touch "$target/.gitignore"
  grep -qxF -- "$line" "$target/.gitignore" || { [[ -z $(tail -c1 "$target/.gitignore") ]] || echo >> "$target/.gitignore"; echo "$line" >> "$target/.gitignore"; }
done < "$skill/manifest.txt"

# The clone's own skill: no skip entries or terms of this project, template hashes of its files.
if [[ ! " ${skipped[*]} " == *" $rel/manifest.txt "* ]]; then sed -i '/^skip /d' "$target/$rel/manifest.txt"; fi
if [[ ! -e "$target/$rel/project-terms.txt" || "$conflict" == overwrite ]]; then
  printf '# Words of this project that must not appear in copied files or templates, one per line.\n' > "$target/$rel/project-terms.txt"
  for v in "$name" "$author" "$repo"; do [[ -z "$v" ]] || echo "$v" >> "$target/$rel/project-terms.txt"; done
fi

echo "cloned into $target"
echo "  copied: $copied, from templates: $templated, empty files: $emptied, symlink CLAUDE.md, .gitignore merged"
if [[ ${#skipped[@]} -gt 0 ]]; then printf '  kept existing: %s\n' "${skipped[*]}"; fi
left=$(cd "$target" && grep -rlE '\{\{(PROJECT_NAME|AUTHOR|REPO_SLUG)\}\}' AGENTS.md docs .claude/agents .claude/commands scripts 2>/dev/null | tr '\n' ' ' || true)
[[ -z "$left" ]] || echo "  unfilled placeholders in: $left"
echo "  still needed: Cargo.toml and Cargo.lock, src/lib.rs + thin src/main.rs, a README with version and coverage badges, git init and the remote"
"$target/$rel/check.sh" --ack-all
