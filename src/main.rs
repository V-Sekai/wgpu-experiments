fn main() {
	pollster::block_on(wgpu_experiments::run())
}
