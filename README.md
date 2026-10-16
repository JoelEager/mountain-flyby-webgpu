# Mountain Flyby WebGPU
A procedurally generated graphics demo implemented in Rust using [WebGPU](https://webgpu.org/) 
which supports both native execution and browsers via WebAssembly. Based on a 
[prior version of this demo which used Vulkan](https://github.com/JoelEager/Mountain-Flyby).

[See it live here](https://joeleager.me/mountain-flyby.html).

## Native Setup
Running the application natively is straightforward as `wgpu` handles the 
graphics backend automatically.

```bash
cargo run
```

## WebAssembly Setup
To run the demo in a browser via WebAssembly, you need [Trunk](https://trunkrs.dev/).
It manages the `wasm-bindgen` and `wasm-opt` toolchains automatically.

1. Install Trunk:
   ```bash
   cargo install trunk --locked
   ```
   (`--locked` prevents dependency resolution issues.)

2. Build and serve with live reload during development:
   ```bash
   trunk serve
   ```
   Trunk builds the WebAssembly, injects the loader into `index.html`, and
   serves it at `http://localhost:8080`.

3. For a release build, run `trunk build --release`, which outputs the
   static site to `dist/`.

## Shaders
The project uses WGSL shaders which are compiled at runtime by `wgpu`.
