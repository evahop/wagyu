struct VertIn {
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec2<f32>,
    @location(1) col: vec3<f32>,
}

struct VertOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec3<f32>,
}

struct Uniform {
    secs: f32,
    size: vec2<u32>,
}

@group(0) @binding(0) var<uniform> u: Uniform;

@vertex
fn vert_main(in: VertIn) -> VertOut {
    let c = cos(u.secs);
    let s = sin(u.secs);
    let rotate = mat2x2(c, s, -s, c);

    let r = vec2(2f) / vec2<f32>(u.size);
    let scale = mat2x2(r.x, 0f, 0f, r.y);

    return VertOut(
        vec4(scale * rotate * in.pos, 0f, 1f),
        in.col,
    );
}

@fragment
fn frag_main(in: VertOut) -> @location(0) vec4<f32> {
    return vec4(in.col, 1f);
}
