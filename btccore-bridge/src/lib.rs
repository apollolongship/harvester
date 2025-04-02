use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use tokio::sync::mpsc::{self, Receiver, Sender};
use url::Url;

/// Using a trait allows us to moch the zmq_receiver
#[async_trait]
pub trait ZmqReceiver {
    async fn recv(&self) -> Result<[u8; 32]>;
}

pub struct Bridge<T: ZmqReceiver> {
    current_header: Option<[u8; 80]>,
    last_block_hash: Option<[u8; 32]>,
    rpc_address: String,
    sender: Sender<[u8; 80]>,
    zmq_address: String,
    zmq_receiver: T,
}

impl<T: ZmqReceiver> Bridge<T> {
    /// Returns a Bridge and a receiver for new headers
    pub fn new(
        rpc_address: &str,
        zmq_address: &str,
        zmq_receiver: T,
    ) -> Result<(Self, Receiver<[u8; 80]>)> {
        // Parsing urls and doing additional checks
        let rpc_url = Url::parse(rpc_address).context("Invalid RPC address.")?;
        if !["http", "https"].contains(&rpc_url.scheme()) {
            return Err(anyhow!("Only http or https are allowed for RPC."));
        }

        let zmq_url = Url::parse(zmq_address).context("Invalid zmq address.")?;
        match zmq_url.scheme() {
            "tcp" => {
                if zmq_url.host().is_none() || zmq_url.port().is_none() {
                    return Err(anyhow!("zmq tcp address must have host and port"));
                }
            }
            "ipc" => {
                if zmq_url.path().is_empty() {
                    return Err(anyhow!("zmq ipc address must have a path"));
                }
            }
            _ => return Err(anyhow!("Only tcp or ipc are allowed for zmq.")),
        }

        let (sender, receiver) = mpsc::channel(8);

        Ok((
            Bridge {
                current_header: None,
                last_block_hash: None,
                rpc_address: rpc_address.to_string(),
                sender,
                zmq_address: zmq_address.to_string(),
                zmq_receiver,
            },
            receiver,
        ))
    }

    pub async fn listen_for_new_block() -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use anyhow::Result;

    struct MockReceiver;

    #[async_trait]
    impl ZmqReceiver for MockReceiver {
        async fn recv(&self) -> anyhow::Result<[u8; 32]> {
            Ok([0u8; 32])
        }
    }

    #[test]
    fn init_bridge_valid_urls() {
        let mock_receiver = MockReceiver;
        let bridge = Bridge::new(
            "http://localhost:8332",
            "tcp://127.0.0.1:28332",
            mock_receiver,
        );
        assert!(bridge.is_ok());

        let mock_receiver = MockReceiver;
        let bridge = Bridge::new(
            "https://minecraft.net:8332",
            "ipc:///tmp/zmq.sock",
            mock_receiver,
        );
        assert!(bridge.is_ok());
    }

    #[test]
    fn init_bridge_invalid_rpc_address_fails() {
        let mock_receiver = MockReceiver;
        let bridge = Bridge::new(
            "ftp://localhost:8332",
            "tcp://127.0.0.1:28332",
            mock_receiver,
        );

        assert!(bridge.is_err());

        let mock_receiver = MockReceiver;
        let bridge = Bridge::new("bad_url", "tcp://127.0.0.1:28332", mock_receiver);

        assert!(bridge.is_err());
    }

    #[test]
    fn init_bridge_invalid_zmq_address_fails() {
        let mock_receiver = MockReceiver;
        let bridge = Bridge::new("http://localhost:8332", "tcp://", mock_receiver);

        assert!(bridge.is_err());

        let mock_receiver = MockReceiver;
        let bridge = Bridge::new("https://localhost:8332", "my_address", mock_receiver);

        assert!(bridge.is_err());
    }
}
