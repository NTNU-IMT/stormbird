#! /bin/bash
cpp_actuator_line_path="../cpp_actuator_line"
cargo build --manifest-path=$cpp_actuator_line_path/Cargo.toml --release

cxxbridge $cpp_actuator_line_path/src/lib.rs --header > src/cpp_actuator_line.hpp
cxxbridge $cpp_actuator_line_path/src/lib.rs > src/cpp_actuator_line.cpp

wmake src