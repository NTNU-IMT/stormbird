#!/bin/bash
CPP_AL_FOLDER=../cpp_actuator_line/

cd $CPP_AL_FOLDER

cargo build --release

cd ../openfoam/

cp $CPP_AL_FOLDER/target/release/libcpp_actuator_line.a rust_build/libcpp_actuator_line.a

cxxbridge $CPP_AL_FOLDER/src/lib.rs --header > src/cpp_actuator_line.hpp
cxxbridge $CPP_AL_FOLDER/src/lib.rs > src/cpp_actuator_line.cpp
