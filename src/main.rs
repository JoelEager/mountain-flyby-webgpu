/// Main entry point of the application.
///
/// All application logic lives in the library crate so that `wasm-pack`
/// (which only builds the `cdylib` target) packages it for the web.
fn main() {
    mountain_flyby_webgpu::run();
}
