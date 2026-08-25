// 粒子运算时屏幕大小是0.0到1000.0，需要映射到-1.0到1.0
struct VertexInput {
    @location(0) position: vec2f,
    @location(1) velocity: vec2f,
    @location(2) acceleration: vec2f,
    @location(3) flag: u32,
    @location(4) _pad: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f,
};

@group(0) @binding(0)
var<uniform> color_array: array<vec4f, 5>;

@vertex
fn vs_main(
    vertices: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4f((vertices.position / 1000.0) * 2.0 - 1.0, 0.0, 1.0);
    
    let p_type = vertices.flag & 0x00000007;
    if p_type < 5 {
        out.color = color_array[p_type];
    } else {
        out.color = vec4f(1.0, 1.0, 1.0, 1.0);
    }
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}

