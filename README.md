# Mountain Flyby
A procedurally generated graphics demo implemented in Rust. Targets [Vulkan](https://docs.vulkan.org/guide/latest/what_is_vulkan.html)
and tested on an AMD RX 6750, though it should compatible with most GPUs. 
Developed using [Google Jules](https://jules.google.com/).

## Setup
The `ash` crate requires that the Vulkan API be present on the path. For 
installation instructions see the crate's read me [here](https://github.com/ash-rs/ash/blob/3eb2ebc6ec35400e69d54ae1ecd862f0999aaba2/README.md#example).

Once that is done you can compile and run the application via `cargo run`.

## Compiling Shaders
The project comes with pre-compiled SPIR-V shaders (`.spv` files), so you don't 
need to compile them to run the demo. However, if you modify the GLSL shaders 
(`.vert` or `.frag`), you will need to recompile them.

You can use `glslc` (included in the Vulkan SDK) to compile the shaders:

```bash
glslangValidator -V shader/terrain.vert -o shader/terrain_vert.spv
glslangValidator -V shader/terrain.frag -o shader/terrain_frag.spv
glslangValidator -V shader/cloud.vert -o shader/cloud_vert.spv
glslangValidator -V shader/cloud.frag -o shader/cloud_frag.spv
```
