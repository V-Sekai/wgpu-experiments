use nalgebra::geometry::{IsometryMatrix3, Perspective3};
use nalgebra::{matrix, Matrix4};

/// OpenGL convention (which nalgebra follows): z goes from [-1, 1].
/// WebGPU uses [0, 1] for z.
const OPENGL_TO_WGPU_M: Matrix4<f32> = matrix![
	1.0, 0.0, 0.0, 0.0;
	0.0, 1.0, 0.0, 0.0;
	0.0, 0.0, 0.5, 0.0;
	0.0, 0.0, 0.5, 1.0;
];

pub struct Camera {
	pub view: IsometryMatrix3<f32>,
	pub proj: Perspective3<f32>,
}
impl Camera {
	/// # Arguments
	/// - `cam_t`: The isometry of the camera, with respect to world
	pub fn proj_view(&self) -> Matrix4<f32> {
		OPENGL_TO_WGPU_M * self.proj.as_matrix() * self.view.to_matrix()
	}
}
