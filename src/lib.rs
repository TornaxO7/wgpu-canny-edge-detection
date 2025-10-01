use wgpu::{include_wgsl, util::DeviceExt};

pub trait Renderer {
    fn device(&self) -> &wgpu::Device;

    fn queue(&self) -> &wgpu::Queue;
}

pub fn detect_edges(_renderer: &dyn Renderer, _img: image::DynamicImage) -> wgpu::Texture {
    todo!()
}

pub fn apply_grayscale(renderer: &dyn Renderer, tv: wgpu::TextureView) -> wgpu::Texture {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let in_texture = tv.texture();
    let out_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gray scale: Output texture"),
        size: in_texture.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: in_texture.dimension(),
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./grayscale.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Grayscale: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Gray scale: Bind group 0"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tv),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Gray scale: Compute pass"),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(
            in_texture.width().div_ceil(WORKGROUP_SIZE),
            in_texture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));

    out_texture
}

pub fn apply_gaussian_filter(
    renderer: &dyn Renderer,
    input_img: image::DynamicImage,
) -> wgpu::Texture {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();
    let img = input_img.to_rgba8();

    let in_texture = {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Gaussian filtered texture"),
            size: wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(img.as_raw()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(img.width() * std::mem::size_of::<[u8; 4]>() as u32),
                rows_per_image: Some(img.height()),
            },
            texture.size(),
        );

        texture
    };

    let out_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gaussian filter: Output tetxure"),
        size: in_texture.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: in_texture.dimension(),
        format: in_texture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let input_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Gaussian filter: Input size buffer"),
        contents: bytemuck::cast_slice(&[in_texture.width() as i32, in_texture.height() as i32]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./shader.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Gaussian filter pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("gaussian_filter"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Gaussian filter: Bind group 0"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &in_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: input_size_buffer.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Gaussian filter: Compute pass"),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(
            in_texture.width().div_ceil(WORKGROUP_SIZE),
            in_texture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));
    out_texture
}

pub fn compute_sobel_operators(
    renderer: &dyn Renderer,
    tv: wgpu::TextureView,
) -> (wgpu::Texture, wgpu::Texture) {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let texture = tv.texture();

    let vertical_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Vertical soeber: Output texture"),
        size: texture.size(),
        mip_level_count: texture.mip_level_count(),
        sample_count: texture.sample_count(),
        dimension: texture.dimension(),
        format: texture.format(),
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });

    let horizontal_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Horizontal soeber: Output texture"),
        size: texture.size(),
        mip_level_count: texture.mip_level_count(),
        sample_count: texture.sample_count(),
        dimension: texture.dimension(),
        format: texture.format(),
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });

    let input_size = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Soeber: Input size"),
        contents: bytemuck::cast_slice(&[texture.width() as i32, texture.height() as i32]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let (vertical_pipeline, horizontal_pipeline) = {
        let shader = device.create_shader_module(include_wgsl!("./shader.wgsl"));

        let vertical_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Vertical Soeber: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("soeber_vertical"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let horizontal_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Horizontal Soeber: Compute pipeline"),
                layout: None,
                module: &shader,
                entry_point: Some("soeber_horizontal"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        (vertical_pipeline, horizontal_pipeline)
    };

    let vertical_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Vertical Soeber: Bind group 0"),
        layout: &vertical_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tv),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &vertical_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: input_size.as_entire_binding(),
            },
        ],
    });

    let horizontal_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Horizontal Soeber: Bind group 0"),
        layout: &horizontal_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tv),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &horizontal_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: input_size.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let dispatch_workgroups_x = texture.width().div_ceil(WORKGROUP_SIZE);
        let dispatch_workgroups_y = texture.height().div_ceil(WORKGROUP_SIZE);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Soeber: Compute pass"),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &vertical_bind_group, &[]);
        pass.set_pipeline(&vertical_pipeline);
        pass.dispatch_workgroups(dispatch_workgroups_x, dispatch_workgroups_y, 1);

        pass.set_bind_group(0, &horizontal_bind_group, &[]);
        pass.set_pipeline(&horizontal_pipeline);
        pass.dispatch_workgroups(dispatch_workgroups_x, dispatch_workgroups_y, 1);
    }

    queue.submit(std::iter::once(encoder.finish()));

    (horizontal_texture, vertical_texture)
}
