use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct Block;
pub struct BlockTemplate;

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
    pub fn new(rpc_client: T) -> Result<(Self, Receiver<[u8; 32]>)> {
        let (sender, receiver) = mpsc::channel(8);

        Ok((
            Bridge {
                block: None,
                rpc_client,
                sender,
            },
            receiver,
        ))
    }

    /// Constructs a block header
    pub async fn construct_header(&self, payout_address: &str) -> [u8; 80] {
        return [0u8; 80];
    }
}

/// Listens for new block indefinitely.
pub async fn listen_for_new_block(
    sender: Sender<[u8; 32]>,
    zmq_receiver: &impl ZmqReceiver,
) -> Result<()> {
    loop {
        let prev_hash = zmq_receiver.recv().await?;
        sender.send(prev_hash).await?;
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
            Ok(BlockTemplate {})
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
        let res = Bridge::new(mock_client);

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn listen_for_new_block_works() {
        let mock_client = MockClient;
        let mock_receiver = MockReceiver;
        let (bridge, mut hash_rx) = Bridge::new(mock_client).expect("Bridge creation failed.");

        let sender = bridge.sender.clone();

        let task = tokio::spawn(async move {
            listen_for_new_block(sender, &mock_receiver)
                .await
                .expect("Listening for block failed.");
        });

        while let Some(_) = hash_rx.recv().await {
            let header = bridge.construct_header("").await;
            assert_eq!(header.len(), 80);
            task.abort();
            break;
        }
    }
}
