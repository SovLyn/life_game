use web_sys::js_sys::Math::random;
use wgpu::util::DeviceExt;

use super::data_structs::SimParams;

use super::data_structs::Vertex;

pub struct LifeGameSimulator {
    particle_num: u32,
    particle_buffer: wgpu::Buffer,
    sim_param_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
}

impl LifeGameSimulator {
    const COLOR_MAP: [[f32; 3]; 6] = [
        [0., 0., 1.],
        [0., 1., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 1.],
        [1., 0., 1.],
    ];

    fn initialize_patticles(n: usize, c: usize) -> Vec<Vertex> {
        (0..n)
            .map(|_| {
                let cla = (random() * c as f64).floor() as usize;

                Vertex {
                    position: [1000.0 * random() as f32, 1000.0 * random() as f32],
                    velocity: [0., 0.],
                    color: Self::COLOR_MAP[cla],
                    flag: cla as _,
                    ..Default::default()
                }
            })
            .collect()
    }

    pub fn new(device: &wgpu::Device, n: usize, c: usize) -> Self {
        if c > 5 {
            panic!("c must be less than 6");
        }
        let initial_particles = Self::initialize_patticles(n, c);

        let particle_num: u32 = initial_particles.len() as u32;

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Particle Buffer"),
            contents: bytemuck::cast_slice(&initial_particles),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::VERTEX,
        });

        let sim_param_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Sim Params Buffer"),
            size: std::mem::size_of::<SimParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/compute.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_param_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            particle_num,
            particle_buffer,
            sim_param_buffer,
            compute_pipeline,
            compute_bind_group,
        }
    }

    pub fn update(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        params: &SimParams,
    ) {
        queue.write_buffer(&self.sim_param_buffer, 0, bytemuck::cast_slice(&[*params]));

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Particle Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

        // dispatch one workgroup per 128 particles
        let workgroup_count = self.particle_num.div_ceil(256);
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    pub fn get_particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    pub fn get_particle_num(&self) -> u32 {
        self.particle_num
    }

}
