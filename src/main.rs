/// Main entry point of the application.
///
/// All application logic lives in the library crate so that Trunk
/// (which builds the `cdylib` target) packages it for the web.
fn main() {
    mountain_flyby_webgpu::run();
}
