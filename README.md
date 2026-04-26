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

3. Create an `index.html` file in the root of the project with the following content:
   ```html
   <!DOCTYPE html>
   <html lang="en">
   <head>
       <meta charset="UTF-8">
       <title>Mountain Flyby</title>
       <style>
           body { margin: 0; padding: 0; background: black; display: flex; justify-content: center; align-items: center; height: 100vh; }
           canvas { display: block; max-width: 100%; max-height: 100vh; }
       </style>
   </head>
   <body id="wasm-example">
       <script type="module">
           import init from './pkg/mountain_flyby.js';
           init();
       </script>
   </body>
   </html>
   ```

4. Serve the directory with an HTTP server (e.g., using Python 3):
   ```bash
   python3 -m http.server 8000
   ```

5. Open your browser and navigate to `http://localhost:8000`.

## Shaders
The project uses WGSL shaders (`.wgsl` files) located in the `shader/` directory. They are compiled at runtime by `wgpu`, so there is no need to manually pre-compile them.
