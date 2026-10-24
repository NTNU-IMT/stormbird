#!/bin/bash
# Meshes and runs the case. OpenFOAM must be loaded in the terminal.
# Usage: bash run.sh [number of processes]
# The default number of processes is the number of physical cores, which excludes hyperthreading.
set -euo pipefail

cd "$(dirname "$0")"

command -v blockMesh >/dev/null || { echo "OpenFOAM not found. Load OpenFOAM in the terminal first."; exit 1; }

physical_cores=$(lscpu -p=Core,Socket 2>/dev/null | grep -v '^#' | sort -u | wc -l || true)

if [ "$physical_cores" -lt 1 ]; then
    physical_cores=$(nproc)
fi

nr_processes=${1:-$physical_cores}
application=$(foamDictionary -entry application -value system/controlDict)

blockMesh > log.blockMesh 2>&1
snappyHexMesh -overwrite > log.snappyHexMesh 2>&1

if [ "$nr_processes" -gt 1 ]; then
    foamDictionary -entry numberOfSubdomains -set "$nr_processes" system/decomposeParDict > /dev/null 2>&1
    decomposePar -force > log.decomposePar 2>&1
    mpirun -np "$nr_processes" "$application" -parallel > "log.$application" 2>&1
    reconstructPar -latestTime > log.reconstructPar 2>&1
    rm -rf processor*
else
    "$application" > "log.$application" 2>&1
fi
