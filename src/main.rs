use image::{ImageBuffer, ImageReader, Luma};
use pollster::FutureExt;
use std::path::Path;
use wgpu_canny_edge_detection::{
    Renderer as RendererTrait, apply_double_thresholding, apply_edge_tracking,
    apply_gaussian_filter, apply_grayscale, apply_magnitude_and_angle,
    apply_non_maximum_suppression, apply_sobel_operators,
};

struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::from_env_or_default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .block_on()
            .expect("Get adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .block_on()
            .unwrap();

        Self { device, queue }
    }

    pub fn save_texture<P: AsRef<Path>>(&self, path: P, texture: &wgpu::Texture) {
        print!("Saving texture...");
        if texture.format() != wgpu::TextureFormat::R32Float {
            panic!("Texture has format: '{:?}'", texture.format());
        }

        let device = self.device();
        let queue = self.queue();

        let size = texture.size();
        let unpadded_bytes_per_row = std::mem::size_of::<f32>() as u32 * size.width;
        let padded_bytes_per_row = unpadded_bytes_per_row.next_multiple_of(256);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output buffer"),
            size: (padded_bytes_per_row * size.height) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Saving command encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(size.height),
                },
            },
            size,
        );

        queue.submit(std::iter::once(encoder.finish()));

        {
            let slice = buffer.slice(..);

            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
            device.poll(wgpu::PollType::Wait).unwrap();
            rx.recv().unwrap().unwrap();

            // now buffer is mapped
            let range = slice.get_mapped_range();
            let r32floats: &[f32] = bytemuck::cast_slice(&range);

            let bytes_per_f32 = std::mem::size_of::<f32>() as u32;
            let floats_per_row = (unpadded_bytes_per_row / bytes_per_f32) as usize;
            let padded_floats_per_row = (padded_bytes_per_row / bytes_per_f32) as usize;

            let mut luma_data: Vec<u8> =
                Vec::with_capacity((floats_per_row * size.height as usize) as usize);

            for row in r32floats.chunks(padded_floats_per_row) {
                // take only the real pixels, skip padded floats at end of row
                let r32floats = &row[..floats_per_row];
                for r32float in r32floats {
                    let luma = r32float.powf(1. / 2.2) * 255.;
                    luma_data.push(luma as u8);
                }
            }

            let image_buffer: ImageBuffer<Luma<u8>, Vec<u8>> =
                ImageBuffer::from_raw(size.width, size.height, luma_data).unwrap();

            image_buffer.save(path).unwrap();
        }

        println!("DONE");
        buffer.unmap();
    }

    fn load_rgba_image(&self, in_img: image::DynamicImage) -> wgpu::Texture {
        print!("Loading texture... ");

        let device = self.device();
        let queue = self.queue();

        let img = in_img.to_rgba8();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Input texture"),
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
            &img.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(std::mem::size_of::<[u8; 4]>() as u32 * img.width()),
                rows_per_image: Some(img.height()),
            },
            texture.size(),
        );

        println!("DONE");
        texture
    }
}

impl wgpu_canny_edge_detection::Renderer for Renderer {
    fn device(&self) -> &wgpu::Device {
        &self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

fn main() {
    let renderer = Renderer::new();

    let mut args = std::env::args();

    // skip binary path
    args.next();

    let input_file = args.next().expect("Input file path is added");
    let output_dir = args.next().expect("Output dir path is added");

    let input = ImageReader::open(input_file).unwrap().decode().unwrap();

    let input_texture = renderer.load_rgba_image(input);

    // 1. gray scaling
    let gray_scale = apply_grayscale(
        &renderer,
        input_texture.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(format!("{output_dir}/1_gray_scale.png"), &gray_scale);

    // 2. Remove noise with gaussian filtering
    let gaussian = apply_gaussian_filter(
        &renderer,
        gray_scale.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(format!("{output_dir}/2_gaussian.png"), &gaussian);

    // 3.1 Detect horizontal and vertical edges
    let (horizontal, vertical) = apply_sobel_operators(
        &renderer,
        gaussian.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(format!("{output_dir}/3_horizontal.png"), &horizontal);
    renderer.save_texture(format!("{output_dir}/3_vertical.png"), &vertical);

    // 3.2 compute gradient magnitude
    let (magnitudes, radians) = apply_magnitude_and_angle(
        &renderer,
        vertical.create_view(&wgpu::TextureViewDescriptor::default()),
        horizontal.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(format!("{output_dir}/4_magnitude.png"), &magnitudes);
    renderer.save_texture(format!("{output_dir}/4_radians.png"), &radians);

    // 4. apply non maximum suppression
    let non_maximum_suppression = apply_non_maximum_suppression(
        &renderer,
        magnitudes.create_view(&wgpu::TextureViewDescriptor::default()),
        radians.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(
        format!("{output_dir}/5_non_maximum_suppression.png"),
        &non_maximum_suppression,
    );

    // 5. Apply upper and lower thresholds
    let threshold_texture = apply_double_thresholding(
        &renderer,
        non_maximum_suppression.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(
        format!("{output_dir}/6_threshold_texture.png"),
        &threshold_texture,
    );

    // 6. edge tracking
    let edge_tracking = apply_edge_tracking(
        &renderer,
        threshold_texture.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    renderer.save_texture(format!("{output_dir}/7_edge_tracking.png"), &edge_tracking);
}
