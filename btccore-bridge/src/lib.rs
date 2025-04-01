use anyhow::{anyhow, Context, Result};
use url::Url;

pub trait ZmqReceiver {
    fn recv(&self) -> Result<Vec<u8>>;
}

pub struct Bridge<T: ZmqReceiver> {
    current_header: Option<[u8; 80]>,
    last_block_hash: Option<String>,
    rpc_address: String,
    zmq_address: String,
    zmq_receiver: T,
}

impl<T: ZmqReceiver> Bridge<T> {
    pub fn new(rpc_address: &str, zmq_address: &str, zmq_receiver: T) -> Result<Self> {
        // Parsing urls and doing additional checks
        let rpc_url = Url::parse(rpc_address).context("Invalid RPC address.")?;
        if !["http", "https"].contains(&rpc_url.scheme()) {
            return Err(anyhow!("Only http or https are allowed for RPC."));
        }

        let zmq_url = Url::parse(zmq_address).context("Invalid ZMQ address.")?;
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
            _ => return Err(anyhow!("Only tcp or ipc are allowed for ZMQ.")),
        }

        Ok(Bridge {
            current_header: None,
            last_block_hash: None,
            rpc_address: rpc_address.to_string(),
            zmq_address: zmq_address.to_string(),
            zmq_receiver,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use anyhow::Result;

    struct MockReceiver;
    impl ZmqReceiver for MockReceiver {
        fn recv(&self) -> anyhow::Result<Vec<u8>> {
            Ok(vec![1, 2, 3])
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
