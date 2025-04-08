# harvester
GPU-accelerated Bitcoin Miner, powered by Rust and wpgu.

## Crates
This repository is the home of 2 standalone crates:
- btccore-bridge
- wgpu-sha256-miner

## btccore-bridge
Communicates with Bitcoin Core by listening for new blocks announced via ZeroMQ messages.
It then calls getblocktemplate via RPC and constructs a header + full block.

## wgpu-sha256-miner
Specialized for hashing 80 byte headers with double SHA256. Takes advantage of the fact that
running a cryptographic algorithm like this is embarrassingly parallel and therefore a
perfect fit for GPU threads.

## Usage
The crates are completely decoupled so you can use them separately. The miner expects a [u8; 80]
and is not dependent on any external types to maximize portability.

If you want to use them together as in harvester-bin, you need to setup the Rpc Client and
ZmqListener and pass them into the Bridge from btccore-bridge. In your main function you would
then set the program to construct a new header (via Bridge) each time a new hash arrives in the
receiver. That is then passed into the miner.

## Compatibility
The miner works for any crypto with a 80 byte header and double SHA256 as the PoW.
Examples include Bitcoin, Bitcoin Cash and Bitcoin SV.
