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

    // 粘度
    particles[index].velocity *= params.alpha;

    // 计算加速度：如果在inner_range内，强烈排斥，否则依照acc_matrix计算
    for (var i: u32 = 0; i < arrayLength(&particles); i++) {
        if (i == index) {
            continue;
        }

        let r = distance(particles[index].position, particles[i].position);

        if (r>= params.outer_range || r < 0.001) {
            continue;
        }

        let other_type = particles[i].flag & 0x00000007;

        if (r < params.inner_range) {
            particles[index].position += normalize(particles[i].position - particles[index].position) * params.inner_range;
        } else {
            particles[index].acceleration += normalize(particles[i].position - particles[index].position) * params.acc_matrix[other_type * 5 + p_type];
        }
    }

    // 更新速度和位置
    particles[index].velocity += 10.0 * (particles[index].acceleration * params.delta_time);
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