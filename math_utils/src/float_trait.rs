//! A trait for floating point numbers.
//! 
//! The main goal is to be able to easily implement functionality that can work for both f32 and 
//! f64, and perhaps make it possible to switch between the two without too much hassle.

use std::ops::{
    Add, Sub, Mul, Div, Rem, Neg, 
    AddAssign, SubAssign, MulAssign, DivAssign, RemAssign
};

pub trait Float: 
    Copy + 
    PartialOrd + 
    Add<Output = Self> + 
    Sub<Output = Self> + 
    Mul<Output = Self> + 
    Div<Output = Self> +
    Rem<Output = Self> +
    Neg<Output = Self> +
    AddAssign +
    SubAssign +
    MulAssign +
    DivAssign +
    RemAssign 
{
    fn from_f64(n: f64) -> Self;
    fn to_f64(self) -> f64;
    
    fn abs(self) -> Self;
    fn sqrt(self) -> Self;
    fn powf(self, n: Self) -> Self;
    fn powi(self, n: i32) -> Self;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn log(self, base: Self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;
    fn atan2(self, other: Self) -> Self;
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn round(self) -> Self;
}


impl Float for f32 {
    fn from_f64(n: f64) -> Self { n as f32 }
    fn to_f64(self) -> f64 { self as f64 }

    fn abs(self) -> Self { self.abs() }
    fn sqrt(self) -> Self { self.sqrt() }
    fn powf(self, n: Self) -> Self { self.powf(n) }
    fn powi(self, n: i32) -> Self { self.powi(n) }
    fn exp(self) -> Self { self.exp() }
    fn ln(self) -> Self { self.ln() }
    fn log(self, base: Self) -> Self { self.log(base) }
    fn sin(self) -> Self { self.sin() }
    fn cos(self) -> Self { self.cos() }
    fn tan(self) -> Self { self.tan() }
    fn asin(self) -> Self { self.asin() }
    fn acos(self) -> Self { self.acos() }
    fn atan(self) -> Self { self.atan() }
    fn atan2(self, other: Self) -> Self { self.atan2(other) }
    fn floor(self) -> Self { self.floor() }
    fn ceil(self) -> Self { self.ceil() }
    fn round(self) -> Self { self.round() }
}

impl Float for f64 {
    fn from_f64(n: f64) -> Self { n }
    fn to_f64(self) -> f64 { self }

    fn abs(self) -> Self { self.abs() }
    fn sqrt(self) -> Self { self.sqrt() }
    fn powf(self, n: Self) -> Self { self.powf(n) }
    fn powi(self, n: i32) -> Self { self.powi(n) }
    fn exp(self) -> Self { self.exp() }
    fn ln(self) -> Self { self.ln() }
    fn log(self, base: Self) -> Self { self.log(base) }
    fn sin(self) -> Self { self.sin() }
    fn cos(self) -> Self { self.cos() }
    fn tan(self) -> Self { self.tan() }
    fn asin(self) -> Self { self.asin() }
    fn acos(self) -> Self { self.acos() }
    fn atan(self) -> Self { self.atan() }
    fn atan2(self, other: Self) -> Self { self.atan2(other) }
    fn floor(self) -> Self { self.floor() }
    fn ceil(self) -> Self { self.ceil() }
    fn round(self) -> Self { self.round() }
}