#!/usr/bin/env bash
# Mechanical consistency check of the human docs. Cheap for agents: it prints one line per problem
# instead of having an agent read every page. Checks:
#  1. relative links in README.md and docs/*.md point to existing files (anchors ignored)
#  2. every docs/<area>-<topic>.md sub-document is linked from its index docs/<area>.md
#  3. no AI, agent, Claude or workflow references in README.md and docs/{business,architecture,code,roadmap}*.md
#  4. every "## " header of docs/changelog.md is "## <version> — <YYYY-MM-DD HH:MM +zone>"; the newest one is the current version
#  5. README "Latest changes" lists at most 3 versions; the poem has 10 verses, verses 1-9 end with a backslash
# Exit 0 when everything passes, 1 with the problems listed.
set -uo pipefail
cd "$(dirname "$0")/.."

check_links() {
  local f dir link
  for f in README.md docs/*.md; do
    dir=$(dirname "$f")
    grep -oE '\]\([^)]+\)' "$f" | sed -E 's/^\]\(//; s/\)$//' | sort -u | while read -r link; do
      case "$link" in http*|mailto:*|'#'*) continue ;; esac
      link=${link%%#*}
      [ -z "$link" ] && continue
      [ -e "$dir/$link" ] || echo "FAIL: $f links to missing $link"
    done
  done
}

check_index() {
  local sub base area index
  for sub in docs/*-*.md; do
    [ -e "$sub" ] || continue
    base=$(basename "$sub")
    area=${base%%-*}
    [ "$area" = andrea ] && area=business  # docs/andrea-choices.md is listed by the business index
    index="docs/$area.md"
    [ -e "$index" ] || { echo "FAIL: $sub has no index $index"; continue; }
    grep -qF "]($base)" "$index" || echo "FAIL: $index does not link sub-document $base"
  done
}

check_terms() {
  local f hits
  for f in README.md docs/business*.md docs/architecture*.md docs/code*.md docs/roadmap.md; do
    [ -e "$f" ] || continue
    hits=$(grep -nE '\bAI\b|\bagents?\b|\bClaude\b|AGENTS|ai-docs|\.claude|agentic|docs/workflow|workflow\.md|/change-workflow|/add-feature' "$f")
    [ -n "$hits" ] && while IFS= read -r h; do echo "FAIL: $f:${h%%:*} forbidden reference: ${h#*:}"; done <<< "$hits"
  done
  return 0
}

check_changelog() {
  local header_re='^## [0-9]+\.[0-9]+\.[0-9]+ — [0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2} [+-][0-9]{4}$'
  local line newest=""
  [ -e docs/changelog.md ] || return 0
  while IFS= read -r line; do
    [[ $line == "## "* ]] || continue
    if [[ ! $line =~ $header_re ]]; then
      echo "FAIL: docs/changelog.md bad header: $line"
    elif [ -z "$newest" ]; then
      newest=${line#\#\# }; newest=${newest%% *}
    fi
  done < docs/changelog.md
  if [ -n "$newest" ] && [ "$newest" != "$(scripts/version.sh get)" ]; then
    echo "FAIL: newest changelog entry $newest is not the current version $(scripts/version.sh get)"
  fi
}

check_readme() {
  local n poem
  n=$(grep -cE '^- \*\*[0-9]+\.[0-9]+\.[0-9]+\*\* — ' README.md)
  [ "$n" -le 3 ] || echo "FAIL: README lists $n versions in Latest changes (max 3)"
  poem=$(sed -n '/^### Ode to the creator/,$p' README.md | sed '1d' | grep -v '^[[:space:]]*$')
  if [ -z "$poem" ]; then
    echo "FAIL: README has no '### Ode to the creator' poem"
    return 0
  fi
  [ "$(echo "$poem" | wc -l)" -eq 10 ] || echo "FAIL: README poem has $(echo "$poem" | wc -l) verses, expected 10"
  echo "$poem" | head -n 9 | grep -qv '\\$' && echo "FAIL: README poem verse 1-9 lacks the trailing backslash"
  echo "$poem" | tail -n 1 | grep -q '\\$' && echo "FAIL: README poem last verse must not end with a backslash"
  return 0
}

report=$( { check_links; check_index; check_terms; check_changelog; check_readme; } )
if [ -n "$report" ]; then
  echo "$report"
  echo "docs-check: $(echo "$report" | wc -l) problem(s)"
  exit 1
fi
echo "docs-check: ok"
