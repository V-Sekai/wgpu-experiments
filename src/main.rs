fn main() -> color_eyre::Result<()> {
	pollster::block_on(wgpu_experiments::run())
}
