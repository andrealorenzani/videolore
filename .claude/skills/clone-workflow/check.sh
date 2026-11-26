#!/usr/bin/env bash
# Keeps /clone-workflow complete. Fails when:
#   - a workflow file is not listed in manifest.txt
#   - a listed file or a template is missing
#   - a templated file changed since its template was last reviewed
#   - a copied file or a template contains a word of project-terms.txt
#   check.sh                     verify
#   check.sh --ack <path>...     record that the template of <path> was reviewed against the file
#   check.sh --ack-all           the same for every template, then verify
set -euo pipefail
skill=$(cd "$(dirname "$0")" && pwd)
cd "$skill/../../.."
rel=.claude/skills/clone-workflow
m="$rel/manifest.txt"
sum() { sha256sum "$1" | cut -c1-16; }
ack() {
  [[ -f "$1" ]] && grep -qE "^template $1( |\$)" "$m" || { echo "not a templated file: $1" >&2; exit 2; }
  sed -i "s|^template $1\( .*\)\{0,1\}\$|template $1 $(sum "$1")|" "$m"
}
case "${1:-}" in
  --ack) shift; for f in "$@"; do ack "$f"; done; exit 0 ;;
  --ack-all) while read -r kind path _; do [[ $kind != template || ! -f $path ]] || ack "$path"; done < <(cat "$m") ;;
  '') ;;
  *) echo "usage: check.sh [--ack <path>... | --ack-all]" >&2; exit 2 ;;
esac

problems=()
listed=(); scan=()
while read -r kind path arg; do
  case "$kind" in
    copy) listed+=("$path"); [[ -e $path ]] || { problems+=("$path: listed as copy but missing"); continue; }
      scan+=("$path") ;;
    template) listed+=("$path"); t="$rel/templates/${path/#./dot-}"
      [[ -f $t ]] || problems+=("$path: template $t missing")
      if [[ ! -f $path ]]; then problems+=("$path: templated file missing")
      elif [[ $(sum "$path") != "$arg" ]]; then problems+=("$path: changed since its template was reviewed (update $t, then check.sh --ack $path)"); fi ;;
    skip) listed+=("$path") ;;
  esac
done < "$m"
scan+=("$rel/templates")

while IFS= read -r f; do
  found=0
  for l in "${listed[@]}"; do
    # shellcheck disable=SC2053  # manifest paths may be globs
    if [[ $f == $l || $f == "$l"/* ]]; then found=1; break; fi
  done
  [[ $found -eq 1 ]] || problems+=("$f: not in $m (add it as copy, template or skip)")
done < <({ find .claude/agents .claude/commands .claude/skills .claude/settings.json scripts .github -type f 2>/dev/null
           ls AGENTS.md LICENSE docs/workflow*.md 2>/dev/null; } | sort)

while IFS= read -r t; do
  d=${t#"$rel/templates/"}; d=${d/#dot-/.}
  grep -qE "^template $d( |\$)" "$m" || problems+=("$t: template without a manifest line")
done < <(find "$rel/templates" -type f | sort)

terms=$(grep -vE '^(#|[[:space:]]*$)' "$rel/project-terms.txt" 2>/dev/null || true)
if [[ -n $terms ]]; then
  while IFS= read -r hit; do problems+=("project word in a generic file: ${hit:0:160}"); done < <(
    grep -rniIF --exclude=project-terms.txt --exclude=manifest.txt -f <(printf '%s\n' "$terms") "${scan[@]}" || true)
fi

if [[ ${#problems[@]} -gt 0 ]]; then printf 'FAIL: %s\n' "${problems[@]}"; echo "clone-workflow check: ${#problems[@]} problem(s)"; exit 1; fi
echo "clone-workflow check: ok"
