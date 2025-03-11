use std::{convert::TryInto, u8};

use hex::FromHexError;
use sha2::{Digest, Sha256};

pub struct GpuMiner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    header_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    batch_size: u32
}

impl GpuMiner {
    pub async fn new(batch_size: u32) -> Self {
        // Standard wgpu setup
        let instance = wgpu::Instance::new(
            &wgpu::InstanceDescriptor::default()
        );

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions::default()
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None
        ).await.unwrap();

        // Load shader
        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor { 
                label: Some("Shader Mine"), 
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("mine.wgsl").into()
                )
            }
        );

        // Buffer to hold header on the GPU
        // Padded buffer is 128 bytes = 1024 bits
        let header_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Header Buffer"),
                size: 128,
                mapped_at_creation: false,
                usage: 
                    wgpu::BufferUsages::STORAGE |
                    wgpu::BufferUsages::COPY_DST,
            }
        );
        
        // Buffer to hold output on the gpu
        let output_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Output Buffer"),
                size: (batch_size * 4) as u64,
                mapped_at_creation: false,
                usage: 
                    wgpu::BufferUsages::STORAGE |
                    wgpu::BufferUsages::COPY_SRC,
            }
        );

        // Staging buffer to map output from CPU
        let staging_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Staging Buffer"),
                size: (batch_size * 4) as u64,
                mapped_at_creation: false,
                usage: 
                    wgpu::BufferUsages::MAP_READ |
                    wgpu::BufferUsages::COPY_DST,
            }
        );

        // Layout for bind group (buffers)
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
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
            ],
        });

        // Create the bind group
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor { 
                label: Some("Bind Group"), 
                layout: &bind_group_layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: header_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                ], 
            }
        );
        
        // Creating our compute pipeline 
        // (represents the whole function of hardware + software)
        let compute_pipeline = device.create_compute_pipeline(&
            wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipe I"),
                layout: Some(&device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Pipeline Layout"),
                        bind_group_layouts: &[&bind_group_layout],
                        push_constant_ranges: &[],
                    })
                ),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        println!("Created GPU Miner.");
        println!("Connected to the following GPU: {:?}", 
            adapter.get_info().name
        );

        GpuMiner {
            device,
            queue,
            compute_pipeline,
            header_buffer,
            output_buffer,
            staging_buffer,
            bind_group,
            batch_size
        }
    }

    pub fn run_batch(&mut self, words: &[u32; 32]) -> Option<u32> {
        // Send header words to buffer
        self.queue.write_buffer(
            &self.header_buffer, 0, bytemuck::cast_slice(words));

        // Command encoder
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor{
                label: Some("Command Encoder")
            }
        );

        // Run the compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(
                &wgpu::ComputePassDescriptor { 
                    label: Some("Compute Pass"), 
                    timestamp_writes: None, 
                }
            );
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(
                &self.batch_size / 256, 1, 1
            );
        }

        // Copy results to staging buffer to read from CPU
        encoder.copy_buffer_to_buffer(
            &self.output_buffer, 0, 
            &self.staging_buffer, 0, (&self.batch_size * 4) as u64
        );
        self.queue.submit(Some(encoder.finish()));
        
        let slice = self.staging_buffer.slice(..);

        slice.map_async(wgpu::MapMode::Read, |res| {
            res.unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let res: Vec<u32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        self.staging_buffer.unmap();
        
        for &nonce in res.iter() {
            if nonce != 0 {
                return Some(nonce);
            }
        } 

        None
    }
}

pub struct BlockHeader {
    version: u32,
    prev_hash: [u8; 32],
    merkle_root: [u8; 32],
    timestamp: u32,
    bits: u32,
} 

/// Holds block header information
impl BlockHeader {
    pub fn new(
        version: &str,
        prev_hash: &str,
        merkle_root: &str,
        timestamp: &str,
        bits: &str
    ) -> Result<Self, FromHexError> {
            let version = Self::hex_to_u32(version)?;
            let prev_hash = Self::hex_to_32_bytes(prev_hash)?;
            let merkle_root = Self::hex_to_32_bytes(merkle_root)?;
            let timestamp = Self::hex_to_u32(timestamp)?;
            let bits =  Self::hex_to_u32(bits)?;

            Ok(
                BlockHeader {version, prev_hash, merkle_root, timestamp, bits}
            )
    }

    /// Converts the header fields and returns them as an [u8; 76]
    pub fn to_bytes(&self) -> [u8; 76] {
            let mut fixed_bytes = [0u8; 76];
            fixed_bytes[0..4].copy_from_slice(&self.version.to_le_bytes());
            fixed_bytes[4..36].copy_from_slice(&self.prev_hash);
            fixed_bytes[36..68].copy_from_slice(&self.merkle_root);
            fixed_bytes[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
            fixed_bytes[72..76].copy_from_slice(&self.bits.to_le_bytes());

            fixed_bytes
    }
    
    // Helper functions
    fn hex_to_u32(s: &str) -> Result<u32, FromHexError> {
        // 4 bytes = 32 bits
        let bytes: [u8; 4] = hex::decode(s)?
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn hex_to_32_bytes(s: &str) -> Result<[u8; 32], FromHexError> {
        hex::decode(s)?
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)
    }
}

pub fn hash_with_nonce(header: &[u8; 80])-> [u8; 32] {
    Sha256::digest(Sha256::digest(header)).into()
}

/// Adds padding to the header to make it 128 bytes (1024 bits)
pub fn sha256_preprocess(header: &[u8; 80]) -> [u8; 128] {
    // Initialize to 128 bytes of 0
    let mut padded = [0u8; 128];
    // First 80 bytes are from the original header
    padded[0..80].copy_from_slice(header);
    
    // Add a byte with 1
    padded[80] = 0x80;

    // Add the length 640 which fits in 2 bytes
    padded[126] = 0x02;
    padded[127] = 0x80;

    padded
}

/// Parse the 32x32-bit words, expects 128 byte header
pub fn sha256_parse_words(header: &[u8; 128]) -> [u32; 32] {
    let mut words = [0u32; 32];
    // Words are chunks of 4 byte = 32 bit
    for (i, chunk) in header.chunks_exact(4).enumerate() {
        words[i] = u32::from_be_bytes(chunk.try_into().unwrap());
    }
    words
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn preprocess_header_is_copied_correctly() {
        let header = [0x01; 80];
        let padded = sha256_preprocess(&header);
        assert_eq!(&padded[0..80], &header);
    }

    #[test]
    fn preprocess_padding_byte_is_set() {
        let header = [0x00; 80];
        let padded = sha256_preprocess(&header);
        assert_eq!(padded[80], 0x80);
    }

    #[test]
    fn preprocess_zero_padding_is_correct() {
        let header = [0xFF; 80];
        let padded = sha256_preprocess(&header);
        for i in 81..120 {
            assert_eq!(padded[i], 0x00);
        }
    }

    #[test]
    fn preprocess_length_field_is_correct() {
        let header = [0x00; 80];
        let padded = sha256_preprocess(&header);
        let expected_length = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x80];
        assert_eq!(&padded[120..128], &expected_length);
    }

    #[test]
    fn preprocess_full_padded_output() {
        let header = [0x01; 80];
        let mut expected = [0x00; 128];
        expected[0..80].copy_from_slice(&header);
        expected[80] = 0x80;
        let length_bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x80];
        expected[120..128].copy_from_slice(&length_bytes);
        let padded = sha256_preprocess(&header);
        assert_eq!(padded, expected);
    }
}
