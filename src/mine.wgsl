// Initial hash values for sha256, 
// first 32 bits of fractional part of first 8 prime numbers
const SHA256_INITIAL_HASH: array<u32, 8> = array<u32, 8>(
    0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
    0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u
);

// SHA-256 Constants, 64 x 32 bit words
// represent the first thirty-two bits of the fractional parts of
// the cube roots of the first sixty-four prime numbers
const K: array<u32, 64> = array<u32, 64>(
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u, 0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u, 0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u, 0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u 
);

fn shr(x: u32, n: u32) -> u32 {
    return x >> n;
}

fn rotr(x: u32, n: u32) -> u32 {
    // Same as SHR but move bits that fall
    // off to the left and OR the 2 results
    return (x >> n) | (x << (32u - n));
}

fn ch(x: u32, y: u32, z: u32) -> u32 {
    // x AND y XOR NOT x AND Z
    return (x & y) ^ ((~x) & z);
}

fn maj(x: u32, y: u32, z: u32) -> u32 {
    // x AND y XOR x AND z XOR y AND z
    return (x & y) ^ (x & z) ^ (y & z);
}

fn bigSigma0(x: u32) -> u32 {
    return rotr(x, 2u) ^ rotr(x, 13u) ^ rotr(x, 22u);
}

fn bigSigma1(x: u32) -> u32 {
    return rotr(x, 6u) ^ rotr(x, 11u) ^ rotr(x, 25u);
}

fn littleSigma0(x: u32) -> u32 {
    return rotr(x, 7u) ^ rotr(x, 18u) ^ shr(x, 3u); 
}

fn littleSigma1(x: u32) -> u32 {
    return rotr(x, 17u) ^ rotr(x, 19u) ^ shr(x, 10u);
}

// Takes 16 intitial words (16*32 = 512 (block))
fn expandMsgSchedule(words: array<u32, 16>) -> array<u32, 64> {
    var schedule: array<u32, 64>;
    // First 16 words are the same from the block
    for(var t = 0u; t < 16u; t = t + 1u) {
	schedule[t] = words[t];
    }
    // Expand according to fips 180-4
    for(var t = 16u; t < 64u; t = t + 1u) {
	schedule[t] =
	    littleSigma1(schedule[t - 2u]) + schedule[t - 7u] +
	    littleSigma0(schedule[t - 15u]) + schedule[t - 16u];
    }

    return schedule;
}

// Computes the hash state
// Also called compression since it goes from 512 bits to 256 bits
fn computeHash(words: array<u32, 16>, hashState: array<u32, 8>) 
    -> array<u32, 8> {
    // Expand message schedule
    var w = expandMsgSchedule(words);

    // Working variables
    var a = hashState[0];
    var b = hashState[1];
    var c = hashState[2];
    var d = hashState[3];
    var e = hashState[4];
    var f = hashState[5];
    var g = hashState[6];
    var h = hashState[7];

    // 64 rounds of compression
    for(var t = 0u; t < 64u; t = t + 1u) {
	var t1 = h + bigSigma1(e) + ch(e,f,g) + K[t] + w[t];
	var t2 = bigSigma0(a) + maj(a,b,c);
	h = g;
	g = f;
	f = e;
	e = d + t1;
	d = c;
	c = b;
	b = a;
	a = t1 + t2;
    }

    // Compute and return i:th intermediate hash values
    return array<u32, 8> (
	a + hashState[0],
	b + hashState[1],
	c + hashState[2],
	d + hashState[3],
	e + hashState[4],
	f + hashState[5],
	g + hashState[6],
	h + hashState[7],
    );
}

@group(0) @binding(0) var<storage, read_write> output: array<u32,8>;

@compute @workgroup_size(1)
fn main() {
    // Input block for an empty string (padded according to SHA-256 rules)
    var inputWords: array<u32, 16> = array<u32, 16>(
        0x80000000u, 0x00000000u, 0x00000000u, 0x00000000u,
        0x00000000u, 0x00000000u, 0x00000000u, 0x00000000u,
        0x00000000u, 0x00000000u, 0x00000000u, 0x00000000u,
        0x00000000u, 0x00000000u, 0x00000000u, 0x00000000u
    );

    var finalHash = computeHash(inputWords, SHA256_INITIAL_HASH);

    for(var i = 0u; i < 8u; i = i + 1u) {
	output[i] = finalHash[i];
    }
}
