use rand::Rng;

pub trait Bitstream {
    fn gen_bits(&mut self, num_bits: u32) -> u64;
}

pub struct RngBitstream<T> {
    rng: T,
    bit_buffer: u64,
    unused_bits: u32,
}

impl<T> RngBitstream<T> {
    pub fn new(rng: T) -> Self {
        RngBitstream {
            rng,
            bit_buffer: 0,
            unused_bits: 0,
        }
    }
}

impl<T: Rng> Bitstream for RngBitstream<T> {
    fn gen_bits(&mut self, num_bits: u32) -> u64 {
        let mut result = 0;
        if self.unused_bits > 0 {
            result |= self.bit_buffer >> (64 - self.unused_bits);
            if num_bits < 64 {
                result &= (1 << num_bits) - 1;
            }
        }
        if num_bits <= self.unused_bits {
            self.unused_bits -= num_bits;
        } else {
            let extra_bits = num_bits - self.unused_bits;
            self.bit_buffer = self.rng.gen();
            result |= self.bit_buffer << self.unused_bits;
            if num_bits < 64 {
                result &= (1 << num_bits) - 1;
            }
            self.unused_bits = 64 - extra_bits;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bitstream, RngBitstream};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaChaRng;

    #[test]
    fn gen_bits_gens_reasonably_distributed_bits() {
        let mut rng = ChaChaRng::seed_from_u64(0);
        let mut bitstream = RngBitstream::new(ChaChaRng::seed_from_u64(0));
        let mut buckets: Vec<_> = (0..=64).map(|_| Vec::new()).collect();
        for _ in 0..1000000 {
            let num_bits = rng.gen_range(0..=64);
            let bits = bitstream.gen_bits(num_bits as u32);
            let mask = if num_bits == 64 {
                u64::MAX
            } else {
                (1 << num_bits) - 1
            };
            assert_eq!(
                bits & mask,
                bits,
                "Bits spilled over ({}):\n{:b}\n{:b}",
                num_bits,
                mask,
                bits
            );
            buckets[num_bits as usize].push(bits);
        }
        for (num_bits, bucket) in buckets.into_iter().enumerate() {
            for bit_index in 0..num_bits {
                let count = bucket
                    .iter()
                    .filter(|&&b| (b & (1 << bit_index)) != 0)
                    .count();
                let frequency = count as f64 / bucket.len() as f64;
                assert!(
                    frequency > 0.4 && frequency < 0.6,
                    "extreme frequency {} at bit index {}/{}",
                    frequency,
                    bit_index,
                    num_bits
                );
            }
        }
    }
}