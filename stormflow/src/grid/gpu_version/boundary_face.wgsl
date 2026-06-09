

struct BoundaryFace {
    axis_offset: u32,
    neighbor_delta: i32,
    shape: vec2<u32>,
    stride: vec2<u32>
}

@group(0) @binding(0) var<uniform> boundary_face: BoundaryFace;
