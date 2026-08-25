use web_sys::js_sys::Math::random;
use wgpu::util::DeviceExt;

use super::data_structs::GridParams;
use super::data_structs::SimParams;
use super::data_structs::Vertex;

// 均匀网格的格子数上限：cell_size >= 5（outer_range 滑杆下限）时，
// 1000x1000 空间最多 200x200 = 40000 格。
const GRID_MAX_CELLS: u32 = 200 * 200;

pub struct LifeGameSimulator {
    particle_num: u32,
    particle_buffer: wgpu::Buffer,
    sim_param_buffer: wgpu::Buffer,
    grid_param_buffer: wgpu::Buffer,
    // 以下 buffer 创建后被 bind group 持有（内部克隆 Arc），字段保留以表达所有权语义
    #[allow(dead_code)]
    particle_cell_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    histogram_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    cell_start_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    cell_cursor_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    sorted_neighbors_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    reset_pipeline: wgpu::ComputePipeline,
    count_pipeline: wgpu::ComputePipeline,
    prefix_pipeline: wgpu::ComputePipeline,
    scatter_pipeline: wgpu::ComputePipeline,
    force_pipeline: wgpu::ComputePipeline,
}

impl LifeGameSimulator {
    fn initialize_patticles(n: usize, c: usize) -> Vec<Vertex> {
        (0..n)
            .map(|_| {
                let cla = (random() * c as f64).floor() as usize;

                Vertex {
                    position: [1000.0 * random() as f32, 1000.0 * random() as f32],
                    velocity: [0., 0.],
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

        let grid_param_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Params Buffer"),
            size: std::mem::size_of::<GridParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let make_storage_buffer = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let particle_cell_buffer =
            make_storage_buffer("Particle Cell Buffer", particle_num as u64 * 4);
        let histogram_buffer =
            make_storage_buffer("Histogram Buffer", GRID_MAX_CELLS as u64 * 4);
        let cell_start_buffer =
            make_storage_buffer("Cell Start Buffer", (GRID_MAX_CELLS + 1) as u64 * 4);
        let cell_cursor_buffer =
            make_storage_buffer("Cell Cursor Buffer", GRID_MAX_CELLS as u64 * 4);
        // Neighbor = position(vec2f) + flag(u32) + pad(u32) = 16 字节
        let sorted_neighbors_buffer =
            make_storage_buffer("Sorted Neighbors Buffer", particle_num as u64 * 16);

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/compute.wgsl").into()),
        });

        // 统一 bind group layout：8 个 binding，所有 pass 共用一个 layout + 一个 bind group。
        let storage_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let uniform_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                storage_entry(0), // particles
                uniform_entry(1), // params
                uniform_entry(2), // grid_params
                storage_entry(3), // particle_cell
                storage_entry(4), // histogram
                storage_entry(5), // cell_start
                storage_entry(6), // cell_cursor
                storage_entry(7), // sorted_neighbors
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: particle_cell_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: histogram_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cell_start_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: cell_cursor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: sorted_neighbors_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let make_pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &compute_shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        Self {
            particle_num,
            particle_buffer,
            sim_param_buffer,
            grid_param_buffer,
            particle_cell_buffer,
            histogram_buffer,
            cell_start_buffer,
            cell_cursor_buffer,
            sorted_neighbors_buffer,
            compute_bind_group,
            reset_pipeline: make_pipeline("reset_histogram"),
            count_pipeline: make_pipeline("count_cells"),
            prefix_pipeline: make_pipeline("prefix_sum"),
            scatter_pipeline: make_pipeline("scatter"),
            force_pipeline: make_pipeline("main"),
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

        // 计算网格参数：cell_size = outer_range（保证数值一致），clamp 下限 5 防越界
        let cell_size = params.outer_range.max(5.0);
        let grid_w = ((1000.0 / cell_size).ceil() as u32).max(1);
        let grid_h = grid_w; // 方形空间
        let grid_params = GridParams {
            grid_w,
            grid_h,
            cell_size,
            particle_num: self.particle_num,
        };
        queue.write_buffer(
            &self.grid_param_buffer,
            0,
            bytemuck::cast_slice(&[grid_params]),
        );

        let grid_cells = grid_w * grid_h;

        let mut pass = |pipeline: &wgpu::ComputePipeline, groups: u32| {
            let mut cp = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });
            cp.set_pipeline(pipeline);
            cp.set_bind_group(0, &self.compute_bind_group, &[]);
            cp.dispatch_workgroups(groups, 1, 1);
        };

        pass(&self.reset_pipeline, grid_cells.div_ceil(256));
        pass(&self.count_pipeline, self.particle_num.div_ceil(256));
        pass(&self.prefix_pipeline, 1);
        pass(&self.scatter_pipeline, self.particle_num.div_ceil(256));
        pass(&self.force_pipeline, self.particle_num.div_ceil(256));
    }

    pub fn get_particle_buffer(&self) -> &wgpu::Buffer {
        &self.particle_buffer
    }

    pub fn get_particle_num(&self) -> u32 {
        self.particle_num
    }
}
