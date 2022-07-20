struct Vert {
    pos: vec2<f32>,
    col: vec3<f32>,
}

struct VertIn {
    @builtin(vertex_index) vi: u32,
}

struct VertOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec3<f32>,
}

@vertex
fn vert_main(in: VertIn) -> VertOut {
    var verts = array<Vert, 3>(
        Vert(vec2( 0f,  1f), vec3(1f, 0f, 0f)),
        Vert(vec2(-1f, -1f), vec3(0f, 1f, 0f)),
        Vert(vec2( 1f, -1f), vec3(0f, 0f, 1f)),
    );

    return VertOut(
        vec4(verts[in.vi].pos, 0f, 1f),
        verts[in.vi].col,
    );
}

@fragment
fn frag_main(in: VertOut) -> @location(0) vec4<f32> {
    return vec4(in.col, 1f);
}
