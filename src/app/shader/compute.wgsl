// =====================================================================
// 粒子生命模拟 —— 均匀网格（uniform grid）加速版
//
// 原始算法是全对全 O(n^2)。这里用 cell_size = outer_range 的均匀网格做
// 空间划分：任意距离 < outer_range 的粒子对必落在同一格或相邻 8 格内
//（因为 |floor(p/R)-floor(q/R)|>=2 => |p-q|>R），因此每个粒子只需遍历
// 3x3 邻域，复杂度从 O(n^2) 降到 O(n·k)（k 为邻域平均粒子数）。
//
// 内层仍保留 dist2 < outer_range^2 门限，力公式与原版完全一致，故
// 邻居集合与全对全等价，仅遍历顺序不同（浮点累加顺序差异 ~1ulp，可忽略）。
//
// 每帧 5 个 pass：
//   1. reset_histogram  清零直方图
//   2. count_cells      算每个粒子的格子坐标并计数
//   3. prefix_sum       直方图 → 排他前缀和（cell_start / cell_cursor）
//   4. scatter          按格子把粒子重排进 sorted_neighbors
//   5. main             3x3 邻域力计算 + 积分 + 边界反弹
// =====================================================================

struct Particle {
    position: vec2f,
    velocity: vec2f,
    acceleration: vec2f,
    flag: u32,
    _pad: u32,
}

// 紧凑的邻居条目（只存力计算所需的字段）：16 字节，减半遍历带宽
struct Neighbor {
    position: vec2f,
    flag: u32,
    _pad: u32,
}

struct SimParams {
    inner_range: f32,
    outer_range: f32,
    delta_time: f32,
    alpha: f32,
    // uniform 数组元素 stride=16，用 vec4f 打包（值在 .x），与 Rust 的 [[f32;4];25] 对齐
    acc_matrix: array<vec4f, 25>,
}

struct GridParams {
    grid_w: u32,
    grid_h: u32,
    cell_size: f32,
    particle_num: u32,
}

// ---- 绑定 ----
@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<uniform> grid_params: GridParams;
@group(0) @binding(3) var<storage, read_write> particle_cell: array<u32>;
@group(0) @binding(4) var<storage, read_write> histogram: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> cell_start: array<u32>;
@group(0) @binding(6) var<storage, read_write> cell_cursor: array<atomic<u32>>;
@group(0) @binding(7) var<storage, read_write> sorted_neighbors: array<Neighbor>;

// 空间 [0, 1000]^2 的格子坐标（按 cell_size 划分，clamp 防越界）
fn pos_to_cell(p: vec2f) -> u32 {
    let cs = grid_params.cell_size;
    let cx = clamp(u32(p.x / cs), 0u, grid_params.grid_w - 1u);
    let cy = clamp(u32(p.y / cs), 0u, grid_params.grid_h - 1u);
    return cy * grid_params.grid_w + cx;
}

// ---- pass 1: 清零直方图 ----
@compute @workgroup_size(256)
fn reset_histogram(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    let g = grid_params.grid_w * grid_params.grid_h;
    if i < g {
        atomicStore(&histogram[i], 0u);
    }
}

// ---- pass 2: 计数 + 记录格子坐标 ----
@compute @workgroup_size(256)
fn count_cells(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if i >= grid_params.particle_num {
        return;
    }
    let c = pos_to_cell(particles[i].position);
    particle_cell[i] = c;
    atomicAdd(&histogram[c], 1u);
}

// ---- pass 3: 排他前缀和（单 workgroup block scan）----
// 直方图最多 200*200 = 40000 项，256 线程（每线程至多 157 项）足够。
// 注：naga 要求 var<workgroup> 声明在模块作用域（函数内只允许 var<function>）
var<workgroup> block_sums: array<u32, 256>;
var<workgroup> block_offsets: array<u32, 256>;

@compute @workgroup_size(256)
fn prefix_sum(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let lid = local_id.x;
    let g = grid_params.grid_w * grid_params.grid_h;
    let per = (g + 255u) / 256u;
    let start = lid * per;
    let end = min(start + per, g);

    // 1) 每个线程对自己的区间做顺序 scan，写"局部排他前缀和"
    var sum: u32 = 0u;
    for (var j = start; j < end; j++) {
        let v = atomicLoad(&histogram[j]);
        cell_start[j] = sum;
        atomicStore(&cell_cursor[j], sum);
        sum += v;
    }
    block_sums[lid] = sum;
    workgroupBarrier();

    // 2) 线程 0 顺序扫描 256 个 block 总和，得到每个 block 的全局偏移
    if lid == 0u {
        var acc: u32 = 0u;
        for (var b = 0u; b < 256u; b++) {
            block_offsets[b] = acc;
            acc += block_sums[b];
        }
        cell_start[g] = acc; // 总粒子数（供 force 读最后一个格子的区间终点）
    }
    workgroupBarrier();

    // 3) 每个线程把自己 block 的全局偏移加到局部前缀和上
    let off = block_offsets[lid];
    for (var j = start; j < end; j++) {
        cell_start[j] += off;
        atomicStore(&cell_cursor[j], cell_start[j]);
    }
}

// ---- pass 4: 散射重排（counting sort 的 scatter 步）----
@compute @workgroup_size(256)
fn scatter(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if i >= grid_params.particle_num {
        return;
    }
    let c = particle_cell[i];
    let slot = atomicAdd(&cell_cursor[c], 1u);
    let p = particles[i];
    sorted_neighbors[slot] = Neighbor(p.position, p.flag & 0x00000007u, 0u);
}

// ---- pass 5: 3x3 邻域力计算 + 积分 + 边界反弹 ----
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let index = gid.x;
    if index >= grid_params.particle_num {
        return;
    }

    let self_pos = particles[index].position;
    let p_type = particles[index].flag & 0x00000007u;
    let self_vel = particles[index].velocity;
    let cell = particle_cell[index];
    let cell_x = cell % grid_params.grid_w;
    let cell_y = cell / grid_params.grid_w;

    let out2 = params.outer_range * params.outer_range;
    let inner = params.inner_range;

    var acc = vec2f(0.0, 0.0);

    for (var jy = 0u; jy < 3u; jy++) {
        for (var jx = 0u; jx < 3u; jx++) {
            let ny = cell_y + jy - 1u;
            let nx = cell_x + jx - 1u;
            if nx >= grid_params.grid_w || ny >= grid_params.grid_h {
                continue;
            }
            let nc = ny * grid_params.grid_w + nx;
            let begin = cell_start[nc];
            let end = cell_start[nc + 1u];
            for (var j = begin; j < end; j++) {
                let n = sorted_neighbors[j];
                let dir = n.position - self_pos;
                let dist2 = dot(dir, dir);
                // 距离为 0（含自身）或超出作用半径则忽略
                if dist2 < 0.000001 || dist2 >= out2 {
                    continue;
                }
                let dist = sqrt(dist2);
                let other_type = n.flag;
                if dist < inner {
                    // 内核软排斥力
                    let strength = inner * (1.0 - dist / inner);
                    acc -= dir / dist * strength;
                } else {
                    // 恒力：规则值 [自己][对方]
                    let g = params.acc_matrix[p_type * 5 + other_type].x;
                    acc += dir / dist * g;
                }
            }
        }
    }

    // 积分
    var vel = self_vel * params.alpha;
    vel += 100.0 * (acc * params.delta_time);
    var pos = self_pos + vel * params.delta_time;

    // 边界反弹，边界为0.0到1000.0
    if pos.x < 0.0 {
        vel.x = -vel.x;
        pos.x = 0.0;
    }
    if pos.x > 1000.0 {
        vel.x = -vel.x;
        pos.x = 1000.0;
    }
    if pos.y < 0.0 {
        vel.y = -vel.y;
        pos.y = 0.0;
    }
    if pos.y > 1000.0 {
        vel.y = -vel.y;
        pos.y = 1000.0;
    }

    particles[index].velocity = vel;
    particles[index].position = pos;
    particles[index].acceleration = acc;
}
