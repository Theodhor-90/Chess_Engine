use crate::arch::NetworkDims;

pub struct Network {
    pub dims: NetworkDims,
    pub input_weights: Box<[i16]>,
    pub input_bias: Box<[i16]>,
    pub hidden1_weights: Box<[i8]>,
    pub hidden1_bias: Box<[i32]>,
    pub hidden2_weights: Box<[i8]>,
    pub hidden2_bias: i32,
}

impl Network {
    pub fn new_zeroed(dims: NetworkDims) -> Self {
        Self {
            dims,
            input_weights: vec![0i16; dims.halfkp_features * dims.l1_size].into_boxed_slice(),
            input_bias: vec![0i16; dims.l1_size].into_boxed_slice(),
            hidden1_weights: vec![0i8; dims.l2_size * 2 * dims.l1_size].into_boxed_slice(),
            hidden1_bias: vec![0i32; dims.l2_size].into_boxed_slice(),
            hidden2_weights: vec![0i8; dims.l2_size].into_boxed_slice(),
            hidden2_bias: 0,
        }
    }

    pub fn dims(&self) -> &NetworkDims {
        &self.dims
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE};

    #[test]
    fn network_new_zeroed() {
        let net = Network::new_zeroed(NetworkDims::default_full());
        assert_eq!(net.input_weights.len(), HALFKP_FEATURES * L1_SIZE);
        assert!(net.input_weights.iter().all(|&w| w == 0));
        assert!(net.input_bias.iter().all(|&b| b == 0));
        assert_eq!(net.hidden1_weights.len(), L2_SIZE * 2 * L1_SIZE);
        assert!(net.hidden1_weights.iter().all(|&w| w == 0));
        assert!(net.hidden1_bias.iter().all(|&b| b == 0));
        assert!(net.hidden2_weights.iter().all(|&w| w == 0));
        assert_eq!(net.hidden2_bias, 0);
    }
}
