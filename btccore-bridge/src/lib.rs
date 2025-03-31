use anyhow::{Context, Result};

pub trait ZmqReceiver {
    fn recv(&self) -> Result<Vec<u8>, String>;
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
    struct MockReceiver;
    impl ZmqReceiver for MockReceiver {
        fn recv(&self) -> Result<Vec<u8>, String> {
            Ok(vec![1, 2, 3])
        }
    }

    #[test]
    fn bridge_is_created() {
        let mock_receiver = MockReceiver;
        let bridge = Bridge::new("1111", "1111", mock_receiver).unwrap();

        assert_eq!(bridge.last_block_hash, None);
        assert_eq!(bridge.current_header, None);
    }
}
