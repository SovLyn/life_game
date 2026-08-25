struct Particle {
    @location(0) position: vec2f,
    @location(1) velocity: vec2f,
    @location(2) acceleration: vec2f,
    @location(3) flag: u32,
    @location(4) color: vec3f,
};

struct SimParams {
    inner_range: f32,
    outer_range: f32,
    delta_time: f32,
    alpha: f32,
    acc_matrix: array<f32, 25>,
}

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> params: SimParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    // Early return if we're out of bounds
    if index >= arrayLength(&particles) {
        return;
    }

    let p_type = particles[index].flag & 0x00000007;

    // 加速度清零
    particles[index].acceleration = vec2f(0.0, 0.0);

    // 原版语义（hunar4321/particle-life）：
    //   力 = 单位方向向量 × acc_matrix[自己][对方]，大小恒定，距离只作门限。
    //   正值 = 指向对方（吸引），负值 = 背离对方（排斥）。
    //   原版没有碰撞检测，粒子允许重叠，靠粘性阻尼稳定，因此这里不做任何位置瞬移。
    for (var i: u32 = 0; i < arrayLength(&particles); i++) {
        if (i == index) {
            continue;
        }

        let dir = particles[i].position - particles[index].position;
        let dist2 = dot(dir, dir);

        // 距离为 0 或超出作用半径则忽略
        if (dist2 < 0.000001 || dist2 >= params.outer_range * params.outer_range) {
            continue;
        }

        let dist = sqrt(dist2);
        let other_type = particles[i].flag & 0x00000007;

        if (dist < params.inner_range) {
            // 内核软排斥力（替代原来的位置瞬移）：
            // 距离越近越强，在 inner_range 处衰减为 0，与外侧恒力平滑衔接。
            // 只负责"别重叠"，仍通过加速度积分，动量守恒、无突跳。
            let strength = params.inner_range * (1.0 - dist / params.inner_range);
            particles[index].acceleration -= dir / dist * strength;
        } else {
            // 恒力：规则值 [自己][对方]
            let g = params.acc_matrix[p_type * 5 + other_type];
            particles[index].acceleration += dir / dist * g;
        }
    }

    // 更新速度和位置
    // 粘性阻尼（原版 v = (v + f) * (1 - viscosity) 的等价形式）
    particles[index].velocity *= params.alpha;
    particles[index].velocity += 1000.0 * (particles[index].acceleration * params.delta_time);
    particles[index].position += particles[index].velocity * params.delta_time;

    // 边界反弹，边界为0.0到1000.0，render中会映射回-1.0到1.0
    if (particles[index].position.x < 0.0) {
        particles[index].velocity.x = -particles[index].velocity.x;
        particles[index].position.x = 0.0;
    }

    if (particles[index].position.x > 1000.0) {
        particles[index].velocity.x = -particles[index].velocity.x;
        particles[index].position.x = 1000.0;
    }

    if (particles[index].position.y < 0.0) {
        particles[index].velocity.y = -particles[index].velocity.y;
        particles[index].position.y = 0.0;
    }

    if (particles[index].position.y > 1000.0) {
        particles[index].velocity.y = -particles[index].velocity.y;
        particles[index].position.y = 1000.0;
    }

}