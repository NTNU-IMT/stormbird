// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Helpers for copying data between the Rust library and arrays owned by the C caller.

use stormath::spatial_vector::SpatialVector;

/// Copies a list of floats into a caller owned array. Returns the number of values written, which
/// is the smallest of the number of values and the length of the output array.
///
/// # Safety
/// `values_out` must point to an array with room for at least `values_out_length` doubles.
pub(crate) fn write_floats(values: &[f64], values_out: *mut f64, values_out_length: usize) -> i32 {
    let actual_count = values.len().min(values_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(values_out, actual_count)
    };

    output_slice.copy_from_slice(&values[0..actual_count]);

    actual_count as i32
}

/// Copies a list of vectors into a caller owned array, as 3 doubles per vector. Returns the number
/// of vectors written, which is the smallest of the number of vectors and `nr_vectors_out`.
///
/// # Safety
/// `vectors_out` must point to an array with room for at least `3 * nr_vectors_out` doubles.
pub(crate) fn write_vectors(
    vectors: &[SpatialVector],
    vectors_out: *mut f64,
    nr_vectors_out: usize
) -> i32 {
    let actual_count = vectors.len().min(nr_vectors_out);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(vectors_out, 3 * actual_count)
    };

    for (i, vector) in vectors.iter().take(actual_count).enumerate() {
        output_slice[3 * i..3 * i + 3].copy_from_slice(&vector.0);
    }

    actual_count as i32
}

/// Reads a vector from 3 doubles in a caller owned array.
///
/// # Safety
/// `vector` must point to an array with at least 3 doubles.
pub(crate) fn read_vector(vector: *const f64) -> SpatialVector {
    let input_slice = unsafe { std::slice::from_raw_parts(vector, 3) };

    SpatialVector::new(input_slice[0], input_slice[1], input_slice[2])
}

/// Reads a list of floats from a caller owned array.
///
/// # Safety
/// `values` must point to an array with at least `length` doubles.
pub(crate) fn read_floats(values: *const f64, length: usize) -> Vec<f64> {
    unsafe { std::slice::from_raw_parts(values, length) }.to_vec()
}
