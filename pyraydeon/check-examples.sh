#!/bin/bash
# Checks that python examples render the expected image
set -eo pipefail

# A cached pyraydeon wheel can mask rust changes entirely; always rebuild.
export UV_NO_CACHE=1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

REQUIRED_TOOLS=("uv" "resvg" "perceptualdiff")
ALL_TOOLS_FOUND=true

echo "Checking that we have necessary tools installed in PATH."
for tool in "${REQUIRED_TOOLS[@]}"; do
    if ! command -v "$tool" &> /dev/null; then
        echo "Error: ❌ $tool is not installed or not in PATH."
        ALL_TOOLS_FOUND=false
    else
        echo "✅ $tool is available."
    fi
done

if [ "$ALL_TOOLS_FOUND" = false ]; then
    echo "Error: Some tools are missing. Please install them and ensure they are in PATH."
    exit 1
fi

for example in ${SCRIPT_DIR}/examples/*.py; do
    example_name=$(basename "$example" .py)

    echo "Running example: $example_name"
    outpath=$(mktemp)

    time uv --project ${SCRIPT_DIR} run ${example} | resvg --resources-dir . - ${outpath}

    outpath_expected=$(mktemp)
    resvg ${SCRIPT_DIR}/examples/${example_name}_expected.svg ${outpath_expected}

    perceptualdiff ${outpath_expected} ${outpath}
    rm "${outpath}" "${outpath_expected}"

    echo "✅ $example_name passed!"
done
