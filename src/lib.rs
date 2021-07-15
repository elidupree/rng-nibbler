use rand::Rng;

pub trait Bitstream {
    fn gen_bits(&mut self, num_bits: u32) -> u64;
}

pub struct RngBitstream<T> {
    rng: T,
    bit_buffer: u64,
    unused_bits: u32,
}

pub struct CountingRngBitstream<T> {
    bitstream: RngBitstream<T>,
    count: u64,
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

impl<T: Rng> Bitstream for CountingRngBitstream<T> {
    fn gen_bits(&mut self, num_bits: u32) -> u64 {
        self.count += num_bits as u64;
        self.bitstream.gen_bits(num_bits)
    }
}

pub trait BitstreamExt {
    fn gen_range(&mut self, size: u64) -> u64;
}

impl<B: Bitstream> BitstreamExt for B {
    fn gen_range(&mut self, size: u64) -> u64 {
        let size_leading_zeros = (size - 1).leading_zeros();
        let bits_needed = 64 - size_leading_zeros;
        let mut leftover: u64 = self.gen_bits(bits_needed);
        if leftover < size {
            return leftover;
        }
        leftover -= size;
        let mut leftover_size: u64 = (1 << bits_needed) - size;
        loop {
            // We need to increase leftover_size to >= size, by adding bits.
            // We could do some fancy leading_zeros thing for this,
            // but the expected value of bits needed given that we reach this code
            // is only something like 2, so the loop is faster.
            let mut bits_needed = 1;
            while (leftover_size << bits_needed) < size {
                bits_needed += 1;
            }
            leftover += self.gen_bits(bits_needed) * leftover_size;
            if leftover < size {
                return leftover;
            }
            leftover_size <<= bits_needed;
            leftover -= size;
            leftover_size -= size;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bitstream, BitstreamExt, CountingRngBitstream, RngBitstream};
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

    #[test]
    fn gen_range_gens_reasonably_distributed_values() {
        let mut rng = ChaChaRng::seed_from_u64(0);
        let mut bitstream = RngBitstream::new(ChaChaRng::seed_from_u64(0));
        let mut buckets: Vec<Vec<u64>> = (0..18)
            .map(|range_size| (0..range_size).map(|_| 0).collect())
            .collect();
        for _ in 0..1000000 {
            let range_size = rng.gen_range(1..18);
            let value = bitstream.gen_range(range_size as u64);
            assert!(value < range_size);
            buckets[range_size as usize][value as usize] += 1;
        }
        dbg!(&buckets);
        for (range_size, bucket) in buckets.into_iter().enumerate() {
            let total_count = bucket.iter().sum::<u64>();
            for (value, &count) in bucket.iter().enumerate() {
                let share = count as f64 * range_size as f64 / total_count as f64;
                assert!(
                    share > 0.9 && share < 1.1,
                    "extreme frequency {} at value {}/{}",
                    share,
                    value,
                    range_size
                );
            }
        }
    }

    #[test]
    fn gen_range_uses_reasonable_bit_counts() {
        for range_size in 1..=17 {
            let mut bitstream = CountingRngBitstream {
                bitstream: RngBitstream::new(ChaChaRng::seed_from_u64(0)),
                count: 0,
            };
            for _ in 0..10000 {
                bitstream.gen_range(range_size as u64);
            }
            dbg!((range_size, bitstream.count));
        }
    }
}
