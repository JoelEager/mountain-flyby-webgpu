# Mountain Flyby
A procedurally generated graphics demo implemented in Rust. Originally targeted Vulkan, now completely rewritten to use WebGPU via `wgpu` to target both native execution and WebAssembly in the browser.
Developed using [Google Jules](https://jules.google.com/).

## Native Setup
Running the application natively is straightforward as `wgpu` handles the graphics backend automatically.

```bash
cargo run
```

## WebAssembly Setup
To run the demo in a browser via WebAssembly, you need to install `wasm-pack`.

1. Install `wasm-pack`:
   ```bash
   cargo install wasm-pack
   ```

2. Build the project for the web:
   ```bash
   wasm-pack build --target web
   ```

3. Serve the directory with an HTTP server (e.g., using Python 3). The project already includes an `index.html` file configured to load the compiled WebAssembly.
   ```bash
   python3 -m http.server 8000
   ```

4. Open your browser and navigate to `http://localhost:8000`.

## Shaders
The project uses WGSL shaders (`.wgsl` files) located in the `shader/` directory. They are compiled at runtime by `wgpu`, so there is no need to manually pre-compile them.
