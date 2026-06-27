use image::DynamicImage;
use wgpu::util::DeviceExt;

// <gpu size threshold>
pub const GPU_PIXEL_THRESHOLD: u32 = 4000 * 2250;
// </gpu size threshold>

// <filter params, mirrors the wgsl FilterParams struct field order and types>
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FilterParams {
    h_min: u32,
    h_max: u32,
    s_min: u32,
    s_max: u32,
    bri_min: u32,
    bri_max: u32,
    width: u32,
    height: u32,
}
// </filter params, mirrors the wgsl FilterParams struct field order and types>

// <gpu context>
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    pub is_discrete: bool,
}
// </gpu context>

// <gpu init, returns none on any failure or insufficient hardware>
pub fn try_init_gpu() -> Option<GpuContext> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;

    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        return None;
    }
    let is_discrete = info.device_type == wgpu::DeviceType::DiscreteGpu;

    let limits = adapter.limits();
    if limits.max_compute_workgroups_per_dimension == 0 {
        return None;
    }

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("image-segmenter-compute"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            ..Default::default()
        },
        None,
    )).ok()?;

    let shader_src = include_str!("shaders/filter.wgsl");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("filter_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("filter_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("filter_pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("filter_pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });

    Some(GpuContext { device, queue, pipeline, bind_group_layout, is_discrete })
}
// </gpu init, returns none on any failure or insufficient hardware>

// <gpu filter dispatch>
pub fn gpu_filter_imagej(
    ctx: &GpuContext,
    img: &DynamicImage,
    h_min: u8, h_max: u8,
    s_min: u8, s_max: u8,
    bri_min: u8, bri_max: u8,
) -> Option<(Vec<u8>, u32)> {
    let rgba = img.to_rgba8();
    let w = rgba.width();
    let h = rgba.height();
    let n_pixels = (w * h) as usize;

    let packed: Vec<u32> = rgba.as_raw()
        .chunks_exact(4)
        .map(|p| (p[0] as u32) | (p[1] as u32) << 8 | (p[2] as u32) << 16)
        .collect();

    let params = FilterParams {
        h_min: h_min as u32, h_max: h_max as u32,
        s_min: s_min as u32, s_max: s_max as u32,
        bri_min: bri_min as u32, bri_max: bri_max as u32,
        width: w, height: h,
    };

    let input_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pixels_in"),
        contents: bytemuck::cast_slice(&packed),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_size = (n_pixels * std::mem::size_of::<u32>()) as u64;
    let output_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mask_out"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let count_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("count_out"),
        contents: bytemuck::cast_slice(&[0u32]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });

    let params_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let staging_mask = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_mask"),
        size: output_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let staging_count = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_count"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("filter_bind_group"),
        layout: &ctx.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: count_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: params_buf.as_entire_binding() },
        ],
    });

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("filter_encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("filter_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&ctx.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (n_pixels as u32 + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_mask, 0, output_size);
    encoder.copy_buffer_to_buffer(&count_buf, 0, &staging_count, 0, 4);
    ctx.queue.submit(Some(encoder.finish()));

    let mask_slice = staging_mask.slice(..);
    let count_slice = staging_count.slice(..);
    let (mask_tx, mask_rx) = std::sync::mpsc::channel();
    let (count_tx, count_rx) = std::sync::mpsc::channel();
    mask_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = mask_tx.send(r); });
    count_slice.map_async(wgpu::MapMode::Read, move |r| { let _ = count_tx.send(r); });

    ctx.device.poll(wgpu::Maintain::Wait);

    mask_rx.recv().ok()?.ok()?;
    count_rx.recv().ok()?.ok()?;

    let mask_data = mask_slice.get_mapped_range().to_vec();
    let count_data = count_slice.get_mapped_range().to_vec();
    let count = u32::from_le_bytes(count_data[0..4].try_into().ok()?);

    Some((mask_data, count))
}
// </gpu filter dispatch>