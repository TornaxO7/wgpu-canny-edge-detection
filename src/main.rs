use image::ImageBuffer;
use image::ImageReader;
use image::Rgba;
use pollster::FutureExt;
use std::path::Path;
use wgpu_canny_edge_detection::Renderer as RendererTrait;
use wgpu_canny_edge_detection::apply_gaussian_filter;

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

    pub fn save_texture<P: AsRef<Path>>(&self, path: P, texture: wgpu::Texture) {
        println!("Saving texture...");
        if texture.format() != wgpu::TextureFormat::Rgba8Unorm {
            panic!(
                "Texture has format: '{:?}' but `Rgba8UnormSrgb` is required.",
                texture.format()
            );
        }

        let device = self.device();
        let queue = self.queue();

        let size = texture.size();

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output buffer"),
            size: ((std::mem::size_of::<[u8; 4]>() as u32 * size.width).next_multiple_of(256)
                * size.height) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let padded_bytes_per_row =
            (std::mem::size_of::<[u8; 4]>() as u32 * size.width).next_multiple_of(256);
        let unpadded_bytes_per_row = std::mem::size_of::<[u8; 4]>() * size.width as usize;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
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
            let mut rgba_data: Vec<u8> =
                Vec::with_capacity(unpadded_bytes_per_row * size.height as usize);

            for row in range.chunks(padded_bytes_per_row as usize) {
                rgba_data.extend_from_slice(&row[..unpadded_bytes_per_row]);
            }

            let image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(size.width, size.height, rgba_data).unwrap();

            image_buffer.save(path).unwrap();
        }

        buffer.unmap();
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
    let output_file = args.next().expect("Output file path is added");

    let input = ImageReader::open(input_file).unwrap().decode().unwrap();

    let texture = apply_gaussian_filter(&renderer, input);

    renderer.save_texture(output_file, texture);
}
