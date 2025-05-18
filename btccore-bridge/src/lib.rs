use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::mpsc::{self, Receiver, Sender};

type Transaction = Vec<u8>;

#[derive(Debug, Default)]
struct CoinbaseTransaction;

/// Full block
pub struct Block {
    header: [u8; 80],
    transactions: Vec<Transaction>,
}

/// Block template as per BIP 0022
/// https://en.bitcoin.it/wiki/BIP_0022
#[derive(Debug, Default)]
pub struct BlockTemplate {
    bits: String,
    curtime: u32,
    height: u32,
    previousblockhash: String,
    sigoplimit: u32,
    sizelimit: u32,
    transactions: Vec<Transaction>,
    version: u8,
    // coinbaseaux is ignored for this implementation
    coinbasetxn: CoinbaseTransaction,
    coinbasevalue: u32,
    // workid is ignored for this implementation
}

/// Trait for dependency injection and mocking
#[async_trait]
pub trait RpcClient {
    async fn getblocktemplate(&self) -> Result<BlockTemplate>;
}

/// Using a trait allows us to mock the zmq_receiver
#[async_trait]
pub trait ZmqReceiver {
    async fn recv(&self) -> Result<[u8; 32]>;
}

/// Bridge between Bitcoin Core and a tokio channel
pub struct Bridge<T: RpcClient> {
    block: Option<Block>,
    rpc_client: T,
    sender: Sender<[u8; 32]>,
}

impl<T: RpcClient> Bridge<T> {
    /// Returns a Bridge and a receiver for new headers
    pub fn new(rpc_client: T) -> (Self, Receiver<[u8; 32]>) {
        let (sender, receiver) = mpsc::channel(8);

        (
            Bridge {
                block: None,
                rpc_client,
                sender,
            },
            receiver,
        )
    }

    /// Updates internal block
    pub async fn update_block(&mut self, payout_address: &str) -> Result<()> {
        let template = self
            .rpc_client
            .getblocktemplate()
            .await
            .context("Couldn't get block template.")?;

        let block = construct_block(template, payout_address);
        let header = block.header;
        self.block = Some(block);

        Ok(())
    }

    /// Getter for block
    pub fn get_block(&self) -> Option<&Block> {
        self.block.as_ref()
    }

    /// Getter for current header
    pub fn get_current_header(&self) -> Option<&[u8; 80]> {
        match self.block.as_ref() {
            Some(block) => Some(&block.header),
            None => None,
        }
    }

    /// Get a clone of the sender
    pub fn get_sender(&self) -> Sender<[u8; 32]> {
        self.sender.clone()
    }
}

/// Listens for new block indefinitely.
pub async fn listen_for_new_block(
    sender: Sender<[u8; 32]>,
    zmq_receiver: impl ZmqReceiver,
) -> Result<()> {
    loop {
        let prev_hash = zmq_receiver
            .recv()
            .await
            .context("Failed to receive ZMQ message.")?;
        sender
            .send(prev_hash)
            .await
            .context("Failed to send message through channel.")?;
    }
}

// Constructs a full block (header + transactions)
fn construct_block(template: BlockTemplate, payout_address: &str) -> Block {
    Block {
        header: [0u8; 80],
        transactions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClient;
    struct MockReceiver;

    #[async_trait]
    impl RpcClient for MockClient {
        async fn getblocktemplate(&self) -> anyhow::Result<BlockTemplate> {
            Ok(BlockTemplate::default())
        }
    }

    #[async_trait]
    impl ZmqReceiver for MockReceiver {
        async fn recv(&self) -> anyhow::Result<[u8; 32]> {
            Ok([0u8; 32])
        }
    }

    #[tokio::test]
    async fn bridge_creation_works() {
        let mock_client = MockClient;
        let (bridge, hash_rx) = Bridge::new(mock_client);

        assert!(bridge.get_block().is_none());
        assert_eq!(hash_rx.capacity(), 8);
    }

    #[tokio::test]
    async fn listen_for_new_block_works() {
        let mock_client = MockClient;
        let mock_receiver = MockReceiver;

        let (mut bridge, mut hash_rx) = Bridge::new(mock_client);
        let sender = bridge.get_sender();

        let task = tokio::spawn(listen_for_new_block(sender, mock_receiver));

        while let Some(_) = hash_rx.recv().await {
            let res = bridge.update_block("").await;
            assert!(res.is_ok());
            let header = bridge.get_current_header().unwrap();
            assert_eq!(header.len(), 80);
            task.abort();
            break;
        }
    }

    #[tokio::test]
    async fn construct_block_works() {}
}
