use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;



pub fn storage_bind_group_layout_entry(binding: usize, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: binding as u32,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub fn uniform_bind_group_layout_entry(binding: usize) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: binding as u32,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Number of workgroups needed to cover `n` elements with a given `workgroup_size` (ceiling
/// division), clamped to at least 1 so dispatches on empty/degenerate axes are still valid.
pub fn workgroup_count(n: usize, workgroup_size: usize) -> u32 {
    (n.div_ceil(workgroup_size)).max(1) as u32
}

/// Flattens vectors to the layout used for vector fields on the GPU: one `f32` per component, 
/// with component `c` of vector `i` at index `3 * i + c`.
pub fn flatten_spatial_vectors(vectors: &[SpatialVector]) -> Vec<Float> {
    let mut out = Vec::with_capacity(3 * vectors.len());

    for vector in vectors {
        out.extend_from_slice(&[vector[0], vector[1], vector[2]]);
    }

    out
}

/// Inverse of [`flatten_spatial_vectors`]
pub fn unflatten_spatial_vectors(values: &[Float]) -> Vec<SpatialVector> {
    values.chunks_exact(3)
        .map(|v| SpatialVector::new(v[0], v[1], v[2]))
        .collect()
}
