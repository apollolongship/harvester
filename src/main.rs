use harvester::{sha256_preprocess, BlockHeader};

#[tokio::main]
async fn main() {
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

    // Creating our compute pipeline (represents the whole function of hardware + software)
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipe I"),
        layout: None,
        module: &shader_module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // Store for the final hash
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: 32,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);

    // Connect output buffer to shader
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
           binding: 0,
           resource: output_buffer.as_entire_binding(),
        }],
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
    
    for word in hash.iter() {
        print!("{:08x}", word);
    }
    println!();

    // Block 884,633
    let _header = BlockHeader::new (
        "02000000",
        "0000000000000000000146601a36528d193ce46aafc00a806b9512663ea89be8",
        "e1419d88433680aeebc7baf6fea1356992cc06b9cb7be7c757a01e003cc78c2b",
        "65d7e920",
        "1700e526"
    ).unwrap();
    
    let header_bytes = [0u8; 80];
    // Add last header to the byte version, nonce left as 0
    //header_bytes[..76].copy_from_slice(&header.to_bytes());

    // Add padding to reach 128 bytes
    let _padded = sha256_preprocess(&header_bytes);

    //println!("{:?}", padded.len());
    //println!("{:?}", padded);
}
