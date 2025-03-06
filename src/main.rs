use core::time;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering}, 
        mpsc, 
        Arc}, 
    thread, 
    u32
};

use harvester::{hash_with_nonce, BlockHeader};

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
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
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

    // Create the buffer, size in bytes
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Buffer I"),
        size: 512,
        usage: 
            wgpu::BufferUsages::STORAGE     | 
            wgpu::BufferUsages::COPY_SRC    |
            wgpu::BufferUsages::COPY_DST    ,
        mapped_at_creation: false,
    });

    // Data to send to the buffer
    // each u32 is 4 bytes, 400 bytes needed!
    let data = vec![0u32; 100];
    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&data));

    // Get bind group layout from shader (via compute pipeline)
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);

    // Create the bind group, it connects the GPU buffer to our shader
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
        label: Some("Bind Group I"), 
        layout: &bind_group_layout, 
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }], 
    });
    
    // Command encoder
    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder I")
    });

    {
        // begin encoding compute pass
        let mut compute_pass = command_encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass I"),
                timestamp_writes: None
            });

        // Setup
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let num_elements = 100;
        let threads_per_workgroup = 64;
        let workgroup_count = (num_elements + threads_per_workgroup - 1) / threads_per_workgroup;

        println!("WORKGROUPS: {workgroup_count}");
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    // Finish command encode into command buffer and push to queue
    let command_buffer = command_encoder.finish();
    queue.submit([command_buffer]);

    // Buffer designed for cpu access
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor{
        label: Some("Stagin Buffer I"),
        size: 512,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // command encoder for copying the GPU buffer to CPU-accessible buffer
    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder II (copy)"),
    });

    // The copy operation in question
    command_encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, 512);

    let command_buffer = command_encoder.finish();
    queue.submit([command_buffer]);

    // Old rune magic
    let slice = staging_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |res| {
        match res {
            Ok(_) => (),
            Err(e) => eprintln!("Failed to map buffer: {:?}", e),
        }
    });
    // Wait for GPU to finish
    device.poll(wgpu::Maintain::Wait);
    {
        let mapped_data = slice.get_mapped_range();
        let data: Vec<u32> = bytemuck::cast_slice(&mapped_data).to_vec();
        println!("RES: \n{:?}", data);
    }

    // Block 884,633
    let header = BlockHeader::new (
        "02000000",
        "0000000000000000000146601a36528d193ce46aafc00a806b9512663ea89be8",
        "e1419d88433680aeebc7baf6fea1356992cc06b9cb7be7c757a01e003cc78c2b",
        "65d7e920",
        "1700e526"
    ).unwrap();

    let cpu_cores = 4;//thread::available_parallelism().unwrap().get();
    println!("Running on {cpu_cores} cores.");

    let mut handles = Vec::with_capacity(cpu_cores);

    let diff = 3;
    let chunk_size = u32::MAX / cpu_cores as u32;

    let mut header_bytes = [0u8; 76];
    header_bytes[..76].copy_from_slice(&header.to_bytes());
    
    let arc_stop = Arc::new(AtomicBool::new(false));
    let arc_header = Arc::new(header_bytes);
    let arc_count = Arc::new(AtomicU64::new(0));

    let (sender, receiver) = mpsc::channel();

    let time_stop = Arc::clone(&arc_stop);
    let time_count = Arc::clone(&arc_count);

    let time_handle = thread::spawn(move || {
        let mut last_count = 0;

        while !time_stop.load(Ordering::Relaxed) {
            thread::sleep(time::Duration::from_secs(3));
            let count = time_count.load(Ordering::Relaxed);

            let hash_rate = (count - last_count) as f64 / 3.0;

            last_count = count;

            println!("Hash rate: {:.2} MH/s. Tried {}k values.", 
                hash_rate / 1_000_000.0, count / 1000);
        }
    });

    for i in 0..cpu_cores {
        let header = Arc::clone(&arc_header);
        let stop = Arc::clone(&arc_stop);
        let sender = sender.clone();
        let count = Arc::clone(&arc_count);

        let handle = thread::spawn(move || {
            let start = chunk_size * i as u32;
            // for the last chunk we want to test up until 2^32 - 1
            let end = if i == cpu_cores-1 {
                u32::MAX 
            } else {
                chunk_size * (i as u32 + 1)
            };

            for nonce in start..end {
                // If stop flag is true, break
                if stop.load(Ordering::Relaxed) {
                    break;
                }

                if (nonce-start) % 1_000 == 0 {
                    count.fetch_add(1000, Ordering::Relaxed);
                }
                // Else we run the hash function
                let mut input = [0u8; 80];
                input[..76].copy_from_slice(&*header);
                input[76..80].copy_from_slice(&nonce.to_le_bytes());
                let hash = hash_with_nonce(&input);
                // Check if we have "diff" leading 0 bytes
                if hash[..diff].iter().all(|&byte| byte == 0) {
                    sender.send((nonce, hash)).unwrap();
                    stop.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });
        handles.push(handle);
    }

    if let Ok((nonce, hash)) = receiver.recv() {
        println!("Found a valid hash!");
        println!("Nonce: {}", nonce);
        println!("Hash: {}", 
            hash.iter()
            .map(|b| format!("{:02x}", b)).collect::<String>()
        );
    }

    time_handle.join().unwrap();

    for h in handles {
        h.join().unwrap();
    }
}
