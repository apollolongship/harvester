use harvester::{sha256_parse_words, sha256_preprocess, BlockHeader};

#[tokio::main]
async fn main() {
    // Block 884,633
    let header = BlockHeader::new (
        "02000000",
        "0000000000000000000146601a36528d193ce46aafc00a806b9512663ea89be8",
        "e1419d88433680aeebc7baf6fea1356992cc06b9cb7be7c757a01e003cc78c2b",
        "65d7e920",
        "1700e526"
    ).unwrap();
    
    let mut header_bytes = [0u8; 80];
    // Add last header to the byte version, nonce left as 0
    header_bytes[..76].copy_from_slice(&header.to_bytes());

    //let hex_string = hex::encode(&header_bytes);
    //println!("{}", hex_string);

    // Add padding to reach 128 bytes
    let padded = sha256_preprocess(&header_bytes);

    let words = sha256_parse_words(&padded);

    // ** WGPU STUFF ** //

    // WGPU instance (navigator.gpu in js/browser)
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    
    // GPUAdapter, like a "bridge" to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await.unwrap();

    // GPUDevice, encapsulates & exposes functionality of device
    // queue is the list of tasks which we build up to run together
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await.unwrap();

    println!("Connected to the following GPU: {:?}", adapter.get_info().name);

    // Load our shader code by creating a shader module
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader I"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mine.wgsl").into()),
    });

    // Layout for bind group (buffers)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    
    // Creating our compute pipeline (represents the whole function of hardware + software)
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipe I"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        })),
        module: &shader_module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Send block words to GPU
    // size is 128 since 128/32 = 4 (size of u32)
    let header_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Header Buffer"),
        size: 128,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(&header_buffer, 0, bytemuck::cast_slice(&words));
    
    // Store for the final hash
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: 32,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Connect output buffer to shader
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: header_buffer.as_entire_binding(),
            },
        ],
    });
    
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    // Compute pass
    {
        let mut compute_pass = encoder.begin_compute_pass(
            &wgpu::ComputePassDescriptor { 
                label: Some("Compute Pass"), 
                timestamp_writes: None }
        );
        
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1);
    }

    // Staging buffer (accessible from CPU)
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, 32);
    queue.submit(Some(encoder.finish()));

    // Map staging buffer and wait for gpu to finish
    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |res| {
        res.unwrap();
    });
    device.poll(wgpu::Maintain::Wait);

    let data = slice.get_mapped_range();
    let hash: [u32; 8] = bytemuck::cast_slice(&data).try_into().unwrap();
    
    println!("GPU:");
    for word in hash.iter() {
        print!("{:08x}", word);
    }
    println!();
}
