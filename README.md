# Mountain Flyby WebGPU
A procedurally generated graphics demo implemented in Rust using [WebGPU](https://webgpu.org/) 
which supports both native execution and browsers via WebAssembly. Based on a 
[prior version of this demo which used Vulkan](https://github.com/JoelEager/Mountain-Flyby). 
Developed using [Google Jules](https://jules.google.com/).

## Native Setup
Running the application natively is straightforward as `wgpu` handles the 
graphics backend automatically.

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

3. Serve the directory with an HTTP server such as Python. The project already 
includes an `index.html` file configured to load the compiled WebAssembly.
   ```bash
   python3 -m http.server 8000
   ```

4. Open your browser and navigate to `http://localhost:8000`.

## Shaders
The project uses WGSL shaders which are compiled at runtime by `wgpu`.
