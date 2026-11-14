// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
/// Optional correction of the angles of attack on the line elements. At the moment, the only
/// option is no correction. The enum is kept so that corrections can be added later without
/// changing the input format.
pub enum AngleOfAttackCorrection {
    /// No correction
    #[default]
    None,
}

