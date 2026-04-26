#![warn(unused_qualifications)]

use std::error::Error;
use std::io::Cursor;

use ash::util::*;
use ash::vk;
use mountain_flyby::*;

mod mesh;

#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy)]
pub struct UniformBufferObject {
    pub mvp: cgmath::Matrix4<f32>,
    pub offset: f32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let window_width = 1920;
    let window_height = 1080;
    let base = VulkanBase::new(window_width, window_height)?;

    unsafe {
        // 1. Pipeline, Renderpass & Framebuffers Setup
        // Define the attachments used in the render pass (color and depth).
        // Render passes describe the attachments (like color and depth buffers)
        // that are used during rendering.
        let renderpass_attachments = [
            vk::AttachmentDescription::default()
                .format(base.surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
            vk::AttachmentDescription::default()
                .format(vk::Format::D16_UNORM)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .initial_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let dependencies = [vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [present_image_view, base.depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                base.device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();

        // 2. Load the 3D model (vertices and indices)
        // Generates the landscape mesh.
        let (terrain_vertices, terrain_index_buffer_data) = mesh::generate_terrain();
        let (cloud_vertices, cloud_index_buffer_data) = mesh::generate_clouds();

        // 3. Setup Index Buffers for drawing
        let (terrain_index_buffer, terrain_index_buffer_memory) = create_device_local_buffer(
            &base,
            &terrain_index_buffer_data,
            vk::BufferUsageFlags::INDEX_BUFFER,
        );
        let (cloud_index_buffer, cloud_index_buffer_memory) = create_device_local_buffer(
            &base,
            &cloud_index_buffer_data,
            vk::BufferUsageFlags::INDEX_BUFFER,
        );

        // 4. Setup Vertex Input Buffers
        let (terrain_vertex_input_buffer, terrain_vertex_input_buffer_memory) =
            create_device_local_buffer(
                &base,
                &terrain_vertices,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            );
        let (cloud_vertex_input_buffer, cloud_vertex_input_buffer_memory) =
            create_device_local_buffer(&base, &cloud_vertices, vk::BufferUsageFlags::VERTEX_BUFFER);

        // 5. Uniform Buffer setup (MVP matrix)
        // Creates a buffer to pass the Model-View-Projection matrix to the shader.
        use cgmath::{Deg, Matrix4, Point3, Vector3};

        let view = Matrix4::look_at_rh(
            // Eye: Position of the camera in world space
            Point3::new(0.0, 5.0, 0.0),
            // Center: Where the camera is looking (pointing slightly up moves model down)
            Point3::new(0.0, 4.0, -10.0),
            // Up: Which way is "up" for the camera
            Vector3::new(0.0, 1.0, 0.0),
        );
        // The last two arguments set the near and far clipping planes
        let mut proj = cgmath::perspective(
            Deg(90.0),
            window_width as f32 / window_height as f32,
            0.1,
            500.0,
        );
        proj[1][1] *= -1.0; // Vulkan Y is down

        let model = Matrix4::from_translation(Vector3::new(0.0, -2.0, 0.0));

        let uniform_color_buffer_data = UniformBufferObject {
            mvp: proj * view * model,
            offset: 0.0,
        };
        let uniform_color_buffer_info = vk::BufferCreateInfo::default()
            .size(size_of_val(&uniform_color_buffer_data) as u64)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let uniform_color_buffer = base
            .device
            .create_buffer(&uniform_color_buffer_info, None)
            .unwrap();
        let uniform_color_buffer_memory_req = base
            .device
            .get_buffer_memory_requirements(uniform_color_buffer);
        let uniform_color_buffer_memory_index = find_memorytype_index(
            &uniform_color_buffer_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the vertex buffer.");

        let uniform_color_buffer_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(uniform_color_buffer_memory_req.size)
            .memory_type_index(uniform_color_buffer_memory_index);
        let uniform_color_buffer_memory = base
            .device
            .allocate_memory(&uniform_color_buffer_allocate_info, None)
            .unwrap();
        let uniform_ptr = base
            .device
            .map_memory(
                uniform_color_buffer_memory,
                0,
                uniform_color_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut uniform_aligned_slice = Align::new(
            uniform_ptr,
            align_of::<UniformBufferObject>() as u64,
            uniform_color_buffer_memory_req.size,
        );
        uniform_aligned_slice.copy_from_slice(&[uniform_color_buffer_data]);
        base.device.unmap_memory(uniform_color_buffer_memory);
        base.device
            .bind_buffer_memory(uniform_color_buffer, uniform_color_buffer_memory, 0)
            .unwrap();

        // 6. Descriptor Pool & Sets allocation (Uniform & Textures)
        // Allocates descriptor sets which bind the uniform buffer to the shader pipeline.
        let descriptor_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        }];
        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_sizes)
            .max_sets(1);

        let descriptor_pool = base
            .device
            .create_descriptor_pool(&descriptor_pool_info, None)
            .unwrap();
        let desc_layout_bindings = [vk::DescriptorSetLayoutBinding::default()
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];
        let descriptor_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&desc_layout_bindings);

        let desc_set_layouts = [base
            .device
            .create_descriptor_set_layout(&descriptor_info, None)
            .unwrap()];

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desc_set_layouts);
        let descriptor_sets = base
            .device
            .allocate_descriptor_sets(&desc_alloc_info)
            .unwrap();

        let uniform_color_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: uniform_color_buffer,
            offset: 0,
            range: size_of_val(&uniform_color_buffer_data) as u64,
        };

        let write_desc_sets = [vk::WriteDescriptorSet::default()
            .dst_set(descriptor_sets[0])
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&uniform_color_buffer_descriptor))];
        base.device.update_descriptor_sets(&write_desc_sets, &[]);

        // 7. Load compiled shaders and create the Graphics Pipeline
        // Reads the SPIR-V shaders, sets up the pipeline layout, and creates the actual graphics pipeline.
        let mut vertex_spv_file = Cursor::new(&include_bytes!("../shader/terrain_vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../shader/terrain_frag.spv")[..]);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        let vertex_shader_module = base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");

        let mut cloud_vertex_spv_file =
            Cursor::new(&include_bytes!("../shader/cloud_vert.spv")[..]);
        let mut cloud_frag_spv_file = Cursor::new(&include_bytes!("../shader/cloud_frag.spv")[..]);

        let cloud_vertex_code =
            read_spv(&mut cloud_vertex_spv_file).expect("Failed to read vertex shader spv file");
        let cloud_vertex_shader_info =
            vk::ShaderModuleCreateInfo::default().code(&cloud_vertex_code);

        let cloud_frag_code =
            read_spv(&mut cloud_frag_spv_file).expect("Failed to read fragment shader spv file");
        let cloud_frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&cloud_frag_code);

        let cloud_vertex_shader_module = base
            .device
            .create_shader_module(&cloud_vertex_shader_info, None)
            .expect("Vertex shader module error");

        let cloud_fragment_shader_module = base
            .device
            .create_shader_module(&cloud_frag_shader_info, None)
            .expect("Fragment shader module error");

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&desc_set_layouts);

        let pipeline_layout = base
            .device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let shader_entry_name = c"main";
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .module(vertex_shader_module)
                .name(shader_entry_name)
                .stage(vk::ShaderStageFlags::VERTEX),
            vk::PipelineShaderStageCreateInfo::default()
                .module(fragment_shader_module)
                .name(shader_entry_name)
                .stage(vk::ShaderStageFlags::FRAGMENT),
        ];
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: std::mem::offset_of!(Vertex, pos) as u32,
        }];
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [base.surface_resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL);

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let noop_stencil_state = vk::StencilOpState::default()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .depth_fail_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS);
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .front(noop_stencil_state)
            .back(noop_stencil_state)
            .max_depth_bounds(1.0);

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B,
            )];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let cloud_color_blend_attachment_states =
            [vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true) // Enable blending for clouds
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let cloud_color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&cloud_color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass);

        let cloud_shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .module(cloud_vertex_shader_module)
                .name(shader_entry_name)
                .stage(vk::ShaderStageFlags::VERTEX),
            vk::PipelineShaderStageCreateInfo::default()
                .module(cloud_fragment_shader_module)
                .name(shader_entry_name)
                .stage(vk::ShaderStageFlags::FRAGMENT),
        ];

        let cloud_graphic_pipeline_infos = graphic_pipeline_infos
            .stages(&cloud_shader_stage_create_infos)
            .color_blend_state(&cloud_color_blend_state);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[graphic_pipeline_infos, cloud_graphic_pipeline_infos],
                None,
            )
            .unwrap();

        let terrain_graphic_pipeline = graphics_pipelines[0];
        let cloud_graphic_pipeline = graphics_pipelines[1];

        // 8. Start the main render loop
        // The loop where the MVP matrix is updated, and each frame is recorded and submitted to the queue.
        let speed = 5.0; // Units per second
        let start_time = std::time::Instant::now();
        let mut current_frame: usize = 0;

        let _ = base.render_loop(|| {
            let elapsed = start_time.elapsed().as_secs_f32();

            // Flyby effect: terrain mesh stays fixed, offset translates the terrain generation inside the shader
            let ubo = UniformBufferObject {
                mvp: uniform_color_buffer_data.mvp,
                offset: -elapsed * speed,
            };
            let mut uniform_aligned_slice = Align::new(
                uniform_ptr,
                align_of::<UniformBufferObject>() as u64,
                uniform_color_buffer_memory_req.size,
            );
            uniform_aligned_slice.copy_from_slice(&[ubo]);

            base.device
                .wait_for_fences(
                    &[base.draw_commands_reuse_fences[current_frame]],
                    true,
                    u64::MAX,
                )
                .expect("Wait for fence failed.");

            base.device
                .reset_fences(&[base.draw_commands_reuse_fences[current_frame]])
                .expect("Reset fences failed.");

            let (present_index, _) = base
                .swapchain_loader
                .acquire_next_image(
                    base.swapchain,
                    u64::MAX,
                    base.present_complete_semaphores[current_frame],
                    vk::Fence::null(),
                )
                .unwrap();
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.3, 0.5, 0.8, 1.0], // Deeper blue background
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(renderpass)
                .framebuffer(framebuffers[present_index as usize])
                .render_area(base.surface_resolution.into())
                .clear_values(&clear_values);

            let draw_command_buffer = base.draw_command_buffers[current_frame];

            base.device
                .reset_command_buffer(
                    draw_command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Reset command buffer failed.");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            base.device
                .begin_command_buffer(draw_command_buffer, &command_buffer_begin_info)
                .expect("Begin commandbuffer");

            base.device.cmd_begin_render_pass(
                draw_command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            base.device.cmd_bind_descriptor_sets(
                draw_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &descriptor_sets[..],
                &[],
            );
            base.device.cmd_bind_pipeline(
                draw_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                terrain_graphic_pipeline,
            );
            base.device
                .cmd_set_viewport(draw_command_buffer, 0, &viewports);
            base.device
                .cmd_set_scissor(draw_command_buffer, 0, &scissors);
            base.device.cmd_bind_vertex_buffers(
                draw_command_buffer,
                0,
                &[terrain_vertex_input_buffer],
                &[0],
            );
            base.device.cmd_bind_index_buffer(
                draw_command_buffer,
                terrain_index_buffer,
                0,
                vk::IndexType::UINT32,
            );
            base.device.cmd_draw_indexed(
                draw_command_buffer,
                terrain_index_buffer_data.len() as u32,
                1,
                0,
                0,
                1,
            );
            base.device.cmd_bind_pipeline(
                draw_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                cloud_graphic_pipeline,
            );
            base.device
                .cmd_set_viewport(draw_command_buffer, 0, &viewports);
            base.device
                .cmd_set_scissor(draw_command_buffer, 0, &scissors);
            base.device.cmd_bind_vertex_buffers(
                draw_command_buffer,
                0,
                &[cloud_vertex_input_buffer],
                &[0],
            );
            base.device.cmd_bind_index_buffer(
                draw_command_buffer,
                cloud_index_buffer,
                0,
                vk::IndexType::UINT32,
            );
            base.device.cmd_draw_indexed(
                draw_command_buffer,
                cloud_index_buffer_data.len() as u32,
                1,
                0,
                0,
                1,
            );
            base.device.cmd_end_render_pass(draw_command_buffer);
            base.device
                .end_command_buffer(draw_command_buffer)
                .expect("End commandbuffer");

            let wait_semaphores = [base.present_complete_semaphores[current_frame]];
            let wait_mask = [vk::PipelineStageFlags::BOTTOM_OF_PIPE];
            let signal_semaphores = [base.rendering_complete_semaphores[present_index as usize]];
            let command_buffers = [draw_command_buffer];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            base.device
                .queue_submit(
                    base.present_queue,
                    &[submit_info],
                    base.draw_commands_reuse_fences[current_frame],
                )
                .expect("queue submit failed.");

            let wait_semaphores = [base.rendering_complete_semaphores[present_index as usize]];
            let swapchains = [base.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);
            base.swapchain_loader
                .queue_present(base.present_queue, &present_info)
                .unwrap();

            current_frame = (current_frame + 1) % 2;
        });
        base.device.device_wait_idle().unwrap();

        // 9. Clean up resources after the window is closed
        for pipeline in graphics_pipelines.iter() {
            base.device.destroy_pipeline(*pipeline, None);
        }
        base.device.destroy_pipeline_layout(pipeline_layout, None);
        base.device
            .destroy_shader_module(vertex_shader_module, None);
        base.device
            .destroy_shader_module(cloud_vertex_shader_module, None);
        base.device
            .destroy_shader_module(fragment_shader_module, None);
        base.device
            .destroy_shader_module(cloud_fragment_shader_module, None);
        base.device.free_memory(terrain_index_buffer_memory, None);
        base.device.free_memory(cloud_index_buffer_memory, None);
        base.device.destroy_buffer(terrain_index_buffer, None);
        base.device.destroy_buffer(cloud_index_buffer, None);
        base.device.free_memory(uniform_color_buffer_memory, None);
        base.device.destroy_buffer(uniform_color_buffer, None);
        base.device
            .free_memory(terrain_vertex_input_buffer_memory, None);
        base.device
            .free_memory(cloud_vertex_input_buffer_memory, None);
        base.device
            .destroy_buffer(terrain_vertex_input_buffer, None);
        base.device.destroy_buffer(cloud_vertex_input_buffer, None);
        for &descriptor_set_layout in desc_set_layouts.iter() {
            base.device
                .destroy_descriptor_set_layout(descriptor_set_layout, None);
        }
        base.device.destroy_descriptor_pool(descriptor_pool, None);
        for framebuffer in framebuffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
        base.device.destroy_render_pass(renderpass, None);

        Ok(())
    }
}
