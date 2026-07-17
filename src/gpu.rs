use image::RgbImage;
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

// <seg edge params, mirrors the wgsl SegParams struct field order and types>
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SegParams {
    tol: u32,
    width: u32,
    height: u32,
    _pad: u32,
}
// </seg edge params, mirrors the wgsl SegParams struct field order and types>

// <gpu context, holds device/queue plus both compute pipelines>
// wgpu's Device/Queue/ComputePipeline/BindGroupLayout do not implement
// Clone directly, so each handle is wrapped in an Arc to make GpuContext
// cheaply cloneable. Deref coercion means call sites that use &ctx.device,
// ctx.queue.submit(...), etc. keep working unchanged.
#[derive(Clone)]
pub struct GpuContext {
    device: std::sync::Arc<wgpu::Device>,
    queue: std::sync::Arc<wgpu::Queue>,
    filter_pipeline: std::sync::Arc<wgpu::ComputePipeline>,
    filter_bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
    seg_pipeline: std::sync::Arc<wgpu::ComputePipeline>,
    seg_bind_group_layout: std::sync::Arc<wgpu::BindGroupLayout>,
    pub is_discrete: bool,
}
// </gpu context, holds device/queue plus both compute pipelines>

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

    // <filter pipeline setup>
    let filter_shader_src = include_str!("shaders/filter.wgsl");
    let filter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("filter_shader"),
        source: wgpu::ShaderSource::Wgsl(filter_shader_src.into()),
    });

    let filter_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("filter_bind_group_layout"),
        entries: &[
            storage_entry(0, true),
            storage_entry(1, false),
            storage_entry(2, false),
            uniform_entry(3),
        ],
    });

    let filter_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("filter_pipeline_layout"),
        bind_group_layouts: &[&filter_bind_group_layout],
        push_constant_ranges: &[],
    });

    let filter_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("filter_pipeline"),
        layout: Some(&filter_pipeline_layout),
        module: &filter_shader,
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });
    // </filter pipeline setup>

    // <segmentation edge pipeline setup>
    let seg_shader_src = include_str!("shaders/seg_edges.wgsl");
    let seg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("seg_edges_shader"),
        source: wgpu::ShaderSource::Wgsl(seg_shader_src.into()),
    });

    let seg_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("seg_bind_group_layout"),
        entries: &[
            storage_entry(0, true),
            storage_entry(1, false),
            uniform_entry(2),
        ],
    });

    let seg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("seg_pipeline_layout"),
        bind_group_layouts: &[&seg_bind_group_layout],
        push_constant_ranges: &[],
    });

    let seg_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("seg_pipeline"),
        layout: Some(&seg_pipeline_layout),
        module: &seg_shader,
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });
    // </segmentation edge pipeline setup>

    Some(GpuContext {
        device: std::sync::Arc::new(device),
        queue: std::sync::Arc::new(queue),
        filter_pipeline: std::sync::Arc::new(filter_pipeline),
        filter_bind_group_layout: std::sync::Arc::new(filter_bind_group_layout),
        seg_pipeline: std::sync::Arc::new(seg_pipeline),
        seg_bind_group_layout: std::sync::Arc::new(seg_bind_group_layout),
        is_discrete,
    })
}
// </gpu init, returns none on any failure or insufficient hardware>

// <bind group layout entry helpers>
fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
// </bind group layout entry helpers>

// <shared readback helper, maps a storage buffer and blocks until data is available>
fn read_buffer(device: &wgpu::Device, buf: &wgpu::Buffer) -> Option<Vec<u8>> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().ok()?.ok()?;
    Some(slice.get_mapped_range().to_vec())
}
// </shared readback helper, maps a storage buffer and blocks until data is available>

// <gpu filter dispatch, takes a pre-converted rgb8 buffer>
pub fn gpu_filter_imagej(
    ctx: &GpuContext,
    rgb: &RgbImage,
    h_min: u8, h_max: u8,
    s_min: u8, s_max: u8,
    bri_min: u8, bri_max: u8,
) -> Option<(Vec<u8>, u32)> {
    let w = rgb.width();
    let h = rgb.height();
    let n_pixels = (w * h) as usize;

    let packed: Vec<u32> = rgb.as_raw()
        .chunks_exact(3)
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
        layout: &ctx.filter_bind_group_layout,
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
        pass.set_pipeline(&ctx.filter_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (n_pixels as u32 + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_mask, 0, output_size);
    encoder.copy_buffer_to_buffer(&count_buf, 0, &staging_count, 0, 4);
    ctx.queue.submit(Some(encoder.finish()));

    let mask_data = read_buffer(&ctx.device, &staging_mask)?;
    let count_data = read_buffer(&ctx.device, &staging_count)?;
    let count = u32::from_le_bytes(count_data[0..4].try_into().ok()?);

    Some((mask_data, count))
}
// </gpu filter dispatch, takes a pre-converted rgb8 buffer>

// <gpu segmentation edge dispatch>
// Computes, for every pixel, whether it is within `tol` of its right and
// down neighbor. Returns one byte per pixel: bit0 = connected right,
// bit1 = connected down. This is the only part of segmentation that runs
// on the GPU, the union-find merge that turns these edges into regions
// runs on the CPU in segment::segment_gpu.
pub fn gpu_compute_seg_edges(ctx: &GpuContext, rgb: &RgbImage, tol: u32) -> Option<Vec<u8>> {
    let w = rgb.width();
    let h = rgb.height();
    let n_pixels = (w * h) as usize;

    let packed: Vec<u32> = rgb.as_raw()
        .chunks_exact(3)
        .map(|p| (p[0] as u32) | (p[1] as u32) << 8 | (p[2] as u32) << 16)
        .collect();

    let params = SegParams { tol, width: w, height: h, _pad: 0 };

    let input_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("seg_pixels_in"),
        contents: bytemuck::cast_slice(&packed),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // output is one u32 per pixel for alignment simplicity, only the low
    // byte is meaningful (values 0..=3)
    let output_size = (n_pixels * std::mem::size_of::<u32>()) as u64;
    let output_buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("seg_edges_out"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let params_buf = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("seg_params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let staging = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("seg_staging"),
        size: output_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("seg_bind_group"),
        layout: &ctx.seg_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: params_buf.as_entire_binding() },
        ],
    });

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("seg_encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("seg_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&ctx.seg_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (n_pixels as u32 + 255) / 256;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging, 0, output_size);
    ctx.queue.submit(Some(encoder.finish()));

    let raw = read_buffer(&ctx.device, &staging)?;

    // unpack u32-per-pixel down to u8-per-pixel edge bitmasks
    let mut edges = Vec::with_capacity(n_pixels);
    for chunk in raw.chunks_exact(4) {
        edges.push(chunk[0]);
    }
    Some(edges)
}
// </gpu segmentation edge dispatch>