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
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
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

pub fn apply_gaussian_filter(renderer: &dyn Renderer, tv: wgpu::TextureView) -> wgpu::Texture {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let in_texture = tv.texture();

    let out_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Gaussian filter: Output tetxure"),
        size: in_texture.size(),
        mip_level_count: in_texture.mip_level_count(),
        sample_count: in_texture.sample_count(),
        dimension: in_texture.dimension(),
        format: in_texture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let kernel = {
        fn gauss(sigma: f32, x: f32, y: f32) -> f32 {
            (1. / (2. * std::f32::consts::PI * sigma * sigma))
                * std::f32::consts::E.powf(-(x * x + y * y) / (2. * sigma * sigma))
        }

        let sigma = 1.6;
        let mut kernel = [[0.; 4]; 3];

        let mut total_sum = 0.;
        for x in (-1)..2 {
            for y in (-1)..2 {
                let value = gauss(sigma, x as f32, y as f32);
                kernel[(x + 1) as usize][(y + 1) as usize] = value;

                total_sum += value;
            }
        }

        // normalize kernel
        for x in (-1)..2 {
            for y in (-1)..2 {
                kernel[(x + 1) as usize][(y + 1) as usize] /= total_sum;
            }
        }

        kernel
    };

    let kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Gaussian filter: Kernel buffer"),
        contents: bytemuck::cast_slice(&kernel),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./kernels.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Gaussian filter pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
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
                resource: kernel_buffer.as_entire_binding(),
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

pub fn apply_sobel_operators(
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
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let horizontal_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Horizontal soeber: Output texture"),
        size: texture.size(),
        mip_level_count: texture.mip_level_count(),
        sample_count: texture.sample_count(),
        dimension: texture.dimension(),
        format: texture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let (vertical_pipeline, horizontal_pipeline) = {
        let shader = device.create_shader_module(include_wgsl!("./kernels.wgsl"));

        let vertical_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Vertical Soeber: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let horizontal_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Horizontal Soeber: Compute pipeline"),
                layout: None,
                module: &shader,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        (vertical_pipeline, horizontal_pipeline)
    };

    let vertical_kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertical Soeber: Kernel"),
        contents: bytemuck::cast_slice(&[-1f32, -2., -1., 0., 0., 0., 0., 0., 1., 2., 1., 0.]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let horizontal_kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Horizontal Soeber: Kernel"),
        contents: bytemuck::cast_slice(&[-1f32, 0., 1., 0., -2., 0., 2., 0., -1., 0., 1., 0.]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

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
                resource: vertical_kernel_buffer.as_entire_binding(),
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
                resource: horizontal_kernel_buffer.as_entire_binding(),
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

pub fn apply_magnitude_and_angle(
    renderer: &dyn Renderer,
    vertical: wgpu::TextureView,
    horizontal: wgpu::TextureView,
) -> (wgpu::Texture, wgpu::Texture) {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let vtexture = vertical.texture();

    let magnitude_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Magnitude: Texture"),
        size: vtexture.size(),
        mip_level_count: vtexture.mip_level_count(),
        sample_count: vtexture.sample_count(),
        dimension: vtexture.dimension(),
        format: vtexture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let radians_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Radians: Texture"),
        size: vtexture.size(),
        mip_level_count: vtexture.mip_level_count(),
        sample_count: vtexture.sample_count(),
        dimension: vtexture.dimension(),
        format: vtexture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./magnitude.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Magnitude: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Magnitude: Bind group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&vertical),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&horizontal),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    &magnitude_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(
                    &radians_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Magnitude: Command encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Magnitude: Compute pass"),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(
            vtexture.width().div_ceil(WORKGROUP_SIZE),
            vtexture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));

    (magnitude_texture, radians_texture)
}

pub fn apply_non_maximum_suppression(
    renderer: &dyn Renderer,
    magnitudes: wgpu::TextureView,
    radians: wgpu::TextureView,
) -> wgpu::Texture {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let m_texture = magnitudes.texture();

    let out_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Non maximum suppression: Texture"),
        size: m_texture.size(),
        mip_level_count: m_texture.mip_level_count(),
        sample_count: m_texture.sample_count(),
        dimension: m_texture.dimension(),
        format: m_texture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./non_maximum_suppression.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Non maximum suppression: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Non maximum suppression: Bind group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&magnitudes),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&radians),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Non maximum suppression: Command encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Non maximum suppression: Compute pass"),
            timestamp_writes: None,
        });

        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(
            m_texture.width().div_ceil(WORKGROUP_SIZE),
            m_texture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));

    out_texture
}

pub fn apply_double_thresholding(
    renderer: &dyn Renderer,
    non_maximum_suppression: wgpu::TextureView,
) -> wgpu::Texture {
    const WORKGROUP_SIZE: u32 = 16;

    let device = renderer.device();
    let queue = renderer.queue();

    let nms_texture = non_maximum_suppression.texture();

    let out_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Double Threshold: Output texture"),
        size: nms_texture.size(),
        mip_level_count: nms_texture.mip_level_count(),
        sample_count: nms_texture.mip_level_count(),
        dimension: nms_texture.dimension(),
        format: nms_texture.format(),
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let threshold_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Double Threshold: Threshold buffer"),
        contents: bytemuck::cast_slice(&[0.3f32, 0.7]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let max_value = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Double Threshold: Max value buffer"),
        contents: bytemuck::bytes_of(&0u32),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let max_value_pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./max_value.wgsl"));
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Double Threshold: Max value pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let threshold_pipeline = {
        let shader = device.create_shader_module(include_wgsl!("./double_threshoulding.wgsl"));
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Double Threshold: Compute pipeline"),
            layout: None,
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };

    let max_value_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Double Threshold: Max value bind group"),
        layout: &max_value_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&non_maximum_suppression),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: max_value.as_entire_binding(),
            },
        ],
    });

    let double_threshold_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Double Threshould: Bind group 0"),
        layout: &threshold_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&non_maximum_suppression),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &out_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: max_value.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: threshold_buffer.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Doule Threshold: Compute pass"),
            timestamp_writes: None,
        });

        // set `max_value` first
        pass.set_bind_group(0, &max_value_bind_group, &[]);
        pass.set_pipeline(&max_value_pipeline);
        pass.dispatch_workgroups(
            nms_texture.width().div_ceil(WORKGROUP_SIZE),
            nms_texture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );

        // now apply thresholds
        pass.set_bind_group(0, &double_threshold_bind_group, &[]);
        pass.set_pipeline(&threshold_pipeline);
        pass.dispatch_workgroups(
            nms_texture.width().div_ceil(WORKGROUP_SIZE),
            nms_texture.height().div_ceil(WORKGROUP_SIZE),
            1,
        );
    }

    queue.submit(std::iter::once(encoder.finish()));

    out_texture
}
