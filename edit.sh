#!/usr/bin/env bash
# ------------------------------------------------------------
# edit.sh  –  case‑sensitive find‑and‑replace in code files
#
# Supported file types: *.rs, *.py, *.js, *.php, *.sh, *.toml, *.md
#
# Usage:
#   ./edit.sh "<search>" "<replace>" <directory>
#
# Example:
#   ./edit.sh "CaseSensative" "replaceCaseSensative" src
#
# ------------------------------------------------------------
set -euo pipefail          # abort on error, undefined var, etc.

# ---------- 1️⃣  Argument handling ----------
if (( $# != 3 )); then
    printf 'Usage: %s "<search>" "<replace>" <directory>\n' "$0" >&2
    exit 1
fi

search=$1
replace=$2
target_dir=$3

# Make sure the directory exists and is a directory
if [[ ! -d $target_dir ]]; then
    printf 'Error: "%s" does not exist or is not a directory.\n' "$target_dir" >&2
    exit 1
fi

# ---------- 2️⃣  Core work ----------
# Find all *.rs, *.py, *.js, *.php, *.sh, *.toml, *.md files under the target directory (recursively)
# -print0 + read -d '' handles filenames with spaces/newlines safely.
find "$target_dir" -type f \( -name '*.rs' -o -name '*.py' -o -name '*.js' -o -name '*.php' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) -print0 |
while IFS= read -r -d '' file; do
    # Use a delimiter that is unlikely to appear in the strings.
    # The ${search} and ${replace} are quoted to keep any spaces intact.
    # The substitution is case‑sensitive by default.
    sed -i '' "s|$search|$replace|g" "$file" 2>/dev/null || \
    sed -i "s|$search|$replace|g" "$file"
done

printf '✅  Replacement finished in %s supported files under "%s".\n' \
       "$(find "$target_dir" -type f \( -name '*.rs' -o -name '*.py' -o -name '*.js' -o -name '*.php' -o -name '*.sh' -o -name '*.toml' -o -name '*.md' \) | wc -l)" "$target_dir"
