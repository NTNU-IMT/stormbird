// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Collection of structs used for input and output in simulations. The structs are kept independent
//! from the specific methods in the library (lifting- and actuator line), as they are used in 
//! multiple places.

pub mod result;
pub mod forces_and_moments;
pub mod derivatives;

pub mod prelude {
    pub use super::result::*;
    pub use super::forces_and_moments::*;
    pub use super::derivatives::*;
}