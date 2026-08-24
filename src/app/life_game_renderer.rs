use web_sys::js_sys::Math::random;
use wgpu::util::DeviceExt;

use super::data_structs::Vertex;

pub struct LifeGameRenderer {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
}

impl LifeGameRenderer {
    pub fn new(device: &wgpu::Device, surface_format: &wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/render.wgsl").into()),
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: *surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let random_vertices: Vec<Vertex> = (0..500)
            .map(|_| Vertex {
                position: [random() as f32 * 2.0 - 1.0, random() as f32 * 2.0 - 1.0],
                color: [random() as f32, random() as f32, random() as f32],
            })
            .collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            // 将顶点数据转换为字节切片
            contents: bytemuck::cast_slice(&random_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            render_pipeline,
            vertex_buffer,
        }
    }
}
