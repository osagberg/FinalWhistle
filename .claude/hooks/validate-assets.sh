#!/usr/bin/env bash
# PreToolUse hook: validate-assets.sh
#
# Event:     PreToolUse (matcher: Write)
# Purpose:   Check Unity asset naming conventions for paths under Assets/.
#              Character prefabs : Character_<Name>.prefab
#              Materials         : M_<Category>_<Name>.mat
#              Textures          : T_<Category>_<Name>_<Suffix>.png
#              Scenes            : <Area>_<Scene>.unity
# Exit:      0 always. Warnings to stderr (advisory).
# Deps:      POSIX grep/sed. python3 preferred for JSON parsing.
#
# stdin JSON: { "tool_name": "Write", "tool_input": { "file_path": "..." } }

INPUT=$(cat)

if command -v python3 >/dev/null 2>&1; then
  FILE_PATH=$(printf '%s' "$INPUT" | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print((d.get("tool_input") or {}).get("file_path",""))
except: pass' 2>/dev/null)
else
  FILE_PATH=$(printf '%s' "$INPUT" | grep -oE '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"file_path"[[:space:]]*:[[:space:]]*"//;s/"$//')
fi

[ -z "$FILE_PATH" ] && exit 0

# Normalize Windows backslashes
FILE_PATH=$(echo "$FILE_PATH" | sed 's|\\|/|g')

# Only check Unity Assets/ tree
case "$FILE_PATH" in
  */Assets/*|Assets/*) ;;
  *) exit 0 ;;
esac

FILENAME=$(basename "$FILE_PATH")
WARNINGS=""

case "$FILENAME" in
  *.prefab)
    # Characters specifically: under Assets/**/Characters/**
    if echo "$FILE_PATH" | grep -qE '(^|/)(Characters|characters)/'; then
      if ! echo "$FILENAME" | grep -qE '^Character_[A-Z][A-Za-z0-9]*\.prefab$'; then
        WARNINGS="$WARNINGS
  PREFAB: character prefab should be 'Character_<Name>.prefab' (got: $FILENAME)"
      fi
    fi
    ;;
  *.mat)
    if ! echo "$FILENAME" | grep -qE '^M_[A-Z][A-Za-z0-9]*_[A-Z][A-Za-z0-9]*\.mat$'; then
      WARNINGS="$WARNINGS
  MATERIAL: expected 'M_<Category>_<Name>.mat' (got: $FILENAME)"
    fi
    ;;
  *.png|*.tga|*.jpg|*.jpeg|*.exr|*.hdr)
    # Textures under Assets/ only
    if ! echo "$FILENAME" | grep -qE '^T_[A-Z][A-Za-z0-9]*_[A-Z][A-Za-z0-9]*(_[A-Z][A-Za-z0-9]*)+\.[A-Za-z]+$'; then
      WARNINGS="$WARNINGS
  TEXTURE: expected 'T_<Category>_<Name>_<Suffix>.<ext>' (got: $FILENAME)"
    fi
    ;;
  *.unity)
    if ! echo "$FILENAME" | grep -qE '^[A-Z][A-Za-z0-9]*_[A-Z][A-Za-z0-9]*\.unity$'; then
      WARNINGS="$WARNINGS
  SCENE: expected '<Area>_<Scene>.unity' (got: $FILENAME)"
    fi
    ;;
esac

if [ -n "$WARNINGS" ]; then
  {
    echo "=== Unity asset naming warnings (advisory) ==="
    echo "$WARNINGS"
    echo "=============================================="
  } >&2
fi

# Never block — naming convention drift is not build-breaking
exit 0
