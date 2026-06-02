

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

pub fn bind_group_entry<'a>(
    binding: usize, 
    buffer: &'a wgpu::Buffer
) -> wgpu::BindGroupEntry<'a> {
    wgpu::BindGroupEntry {
        binding: binding as u32,
        resource: buffer.as_entire_binding(),
    }
}
