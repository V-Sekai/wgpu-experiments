use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Pos {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}
impl Pos {
	pub const fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x, y, z }
	}
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Rgb {
	pub r: f32,
	pub g: f32,
	pub b: f32,
}
impl Rgb {
	pub const fn new(r: f32, g: f32, b: f32) -> Self {
		Self { r, g, b }
	}
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
	pub pos: Pos,
	pub color: Rgb,
}
impl Vertex {
	pub const fn new(pos: Pos, color: Rgb) -> Self {
		Vertex { pos, color }
	}
}
