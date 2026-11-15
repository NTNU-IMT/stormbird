#!/bin/bash
set -euo pipefail

command -v cxxbridge >/dev/null || { echo "cxxbridge not found. Run: cargo install cxxbridge-cmd"; exit 1; }
command -v wmake >/dev/null || { echo "wmake not found. Load OpenFOAM in the terminal first."; exit 1; }

# The OpenFOAM version (ESI/Keysight or Foundation) is detected from the loaded environment
echo "Building the actuator line interface for OpenFOAM $WM_PROJECT_VERSION"

CPP_AL_FOLDER=../cpp_actuator_line/

cd $CPP_AL_FOLDER

cargo build --release

cd ../openfoam/

cp $CPP_AL_FOLDER/target/release/libcpp_actuator_line.a rust_build/libcpp_actuator_line.a

cxxbridge $CPP_AL_FOLDER/src/lib.rs --header > src/cpp_actuator_line.hpp
cxxbridge $CPP_AL_FOLDER/src/lib.rs > src/cpp_actuator_line.C

# Both OpenFOAM versions use the same object folder, so remove any objects built for another version
wclean src

wmake src
