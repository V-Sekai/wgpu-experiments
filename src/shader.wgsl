
struct CameraUniform {
	view_proj: mat4x4<f32>
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) pos: vec3<f32>,
	@location(1) uv: vec2<f32>,
};

struct VertexOutput {
	@builtin(position) clip_pos: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
	verts: VertexInput
) -> VertexOutput {
	var out: VertexOutput;
	out.uv = verts.uv;
	out.clip_pos = camera.view_proj * vec4<f32>(verts.pos, 1.0);
	return out;
}


@group(0) @binding(0)
var diffuse_t: texture_2d<f32>;
@group(0) @binding(1)
var diffuse_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	return textureSample(diffuse_t, diffuse_s, in.uv);
}
