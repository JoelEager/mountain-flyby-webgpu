use mountain_flyby::run;

/// Main entry point of the application.
///
/// Uses `pollster` to execute the asynchronous `run` function synchronously,
/// which handles setting up the event loop, window, and rendering state.
fn main() {
    pollster::block_on(run());
}
