#!/bin/bash

cargo build --release

cxxbridge src/lib.rs --header > src_cpp/cpp_actuator_line.hpp
cxxbridge src/lib.rs > src_cpp/cpp_actuator_line.cpp

wmake src