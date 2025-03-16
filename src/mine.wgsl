/// Naga doesn't support import yet
/// import "sha256.wgsl" as sha256;
/// So we have to manually concat files for now.
@group(0) @binding(0) var<storage, read> headerWords: array<u32, 32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32, 1000000>;

// wg_size needs to be set manually from CPU-side
@compute @workgroup_size({{wg_size}})
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let mTarget: array<u32, 8> = array<u32, 8>(
	0x00000000u,  // 2 zero bytes 
	0xFFFFFFFFu,
	0xFFFFFFFFu,
	0xFFFFFFFFu,
	0xFFFFFFFFu,
	0xFFFFFFFFu,
	0xFFFFFFFFu,
	0xFFFFFFFFu 
    );

    let thId = id.x;
    // Lucky Number?
    let baseNonce = 777777u;
    
    // Nonce on this invocation: base + id
    let nonce: u32 = baseNonce + thId;

    var words: array<u32, 32>;
    // The words should be copied
    for(var i = 0u; i < 32u; i = i + 1u) {
	words[i] = headerWords[i];
    }

    // The nonce is in bytes 76-80 in the btc header
    // 76 / 4 = 19 (each location in words is 4 bytes)
    words[19] = nonce;
    
    var finalHash = doubleHash(words);
    var meetsTarget = true;

    for(var i = 0u; i < 8u; i = i + 1u) {
	if(finalHash[i] > mTarget[i]) {
	    meetsTarget = false;
	    break;
	}
    }

    if(meetsTarget) {
	output[thId] = nonce;
    } else {
	output[thId] = 0u;
    }
}
