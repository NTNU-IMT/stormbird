// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Serializer, Deserialize, Deserializer};

use super::*;

#[derive(Serialize, Deserialize)]
struct Vec3 {
    x: Float,
    y: Float,
    z: Float,
}

impl From<&SpatialVector> for Vec3 {
    fn from(vec: &SpatialVector) -> Self {
        Self {
            x: vec.0[0],
            y: vec.0[1],
            z: vec.0[2],
        }
    }
}

impl Serialize for SpatialVector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec3: Vec3 = self.into();

        vec3.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SpatialVector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec3 = Vec3::deserialize(deserializer)?;

        Ok(SpatialVector::new(
            vec3.x,
            vec3.y,
            vec3.z
        ))
    }
}