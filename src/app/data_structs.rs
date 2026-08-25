#[repr(C)]
// 布局说明（必须与 compute.wgsl 中的 struct Particle 完全一致，否则
// compute 与 render 读写同一缓冲区时元素 stride 会错位）
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub acceleration: [f32; 2],
    // 3bit: type;
    // 1bit: is_discarded;
    pub flag: u32,
    pub _padding: u32,
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: core::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: core::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: core::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: (core::mem::size_of::<[f32; 6]>() + core::mem::size_of::<u32>())
                        as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct SimParams {
    pub inner_range: f32,
    pub outer_range: f32,
    pub delta_time: f32,
    pub alpha: f32,
    // WGSL uniform 的 array 要求元素 stride=16，故用 [[f32;4];25]（每个 16 字节）
    // 与 compute.wgsl 的 array<vec4f,25> 逐字节对齐；每个 [f32;4] 的 .x 存有效值。
    pub acc_matrix: [[f32; 4]; 25],
}
