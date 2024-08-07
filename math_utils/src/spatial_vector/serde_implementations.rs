// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Serializer, Deserialize, Deserializer};

use super::*;

#[derive(Serialize, Deserialize)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Serialize, Deserialize)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl From<&SpatialVector<3>> for Vec3 {
    fn from(vec: &SpatialVector<3>) -> Self {
        Self {
            x: vec.0[0],
            y: vec.0[1],
            z: vec.0[2],
        }
    }
}

impl From<&SpatialVector<2>> for Vec2 {
    fn from(vec: &SpatialVector<2>) -> Self {
        Self {
            x: vec.0[0],
            y: vec.0[1],
        }
    }
}

impl Serialize for SpatialVector<3> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec3: Vec3 = self.into();

        vec3.serialize(serializer)
    }
}

impl Serialize for SpatialVector<2> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec2: Vec2 = self.into();

        vec2.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SpatialVector<3> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec3 = Vec3::deserialize(deserializer)?;

        Ok(SpatialVector::<3>::new(vec3.x, vec3.y, vec3.z))
    }
}

impl<'de> Deserialize<'de> for SpatialVector<2> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec2 = Vec2::deserialize(deserializer)?;

        Ok(SpatialVector::<2>::new(vec2.x, vec2.y))
    }
}