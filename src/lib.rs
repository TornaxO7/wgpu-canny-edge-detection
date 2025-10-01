use wgpu::{include_wgsl, util::DeviceExt};

pub trait Renderer {
    fn device(&self) -> &wgpu::Device;

    fn queue(&self) -> &wgpu::Queue;
}

pub fn detect_edges(renderer: &dyn Renderer, img: image::DynamicImage) -> wgpu::Texture {
    let _filtered_texture = apply_gaussian_filter(renderer, img);
    todo!()
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
        let shader = device.create_shader_module(include_wgsl!("./gaussian_filter.wgsl"));

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Gaussian filter pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
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
