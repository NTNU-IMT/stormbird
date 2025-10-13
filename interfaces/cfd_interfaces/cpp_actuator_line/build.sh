#!/bin/bash
OPENFOAM_FOLDER=../openfoam/

cargo build --release

cp target/release/libcpp_actuator_line.a $OPENFOAM_FOLDER/rust_build/libcpp_actuator_line.a

cxxbridge src/lib.rs --header > $OPENFOAM_FOLDER/src/cpp_actuator_line.hpp
cxxbridge src/lib.rs > $OPENFOAM_FOLDER/src/cpp_actuator_line.cpp