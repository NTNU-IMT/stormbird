# Stormbird

This crate contains the core library written in Rust. See [Super::README](../README.md) for a more high-level 
introduction.

## Use Instructions
The most typical use case of the rust library is to use it in some other library or executable. 

Examples of this can be found [pystormbird](../pystormbird) and the [FMUs](../fmus) in the main Stormbird repo.

## Build instructions
To do a test build, run:

`cargo build`

## Tests
The library includes several tests that are mean to check the most basic functionality. To execute them, run: 

`cargo test`

## Code Documentation
To generate and open the code documentation, run:

`cargo doc --no-deps --open`