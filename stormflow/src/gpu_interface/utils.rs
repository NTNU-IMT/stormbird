

pub fn storage_bind_group_layout_entry(binding: usize, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: binding as u32,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: read_only },
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
