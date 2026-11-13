/// Gerador pseudoaleatório SplitMix64: pequeno, rápido e igual em toda
/// plataforma, para a mesma seed dar a mesma partida.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    /// Gerador a partir da seed.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Próximo valor de 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Valor em `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits: o que cabe exato na mantissa de um f32.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Inteiro em `min..=max`; com `max < min` devolve `min`.
    pub fn range_u32(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let span = u64::from(max - min) + 1;
        min + (self.next_u64() % span) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(7).next_u64(), Rng::new(8).next_u64());
    }

    #[test]
    fn f32_stays_in_unit_interval() {
        let mut r = Rng::new(1);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v), "{v}");
        }
    }

    #[test]
    fn range_is_inclusive_and_bounded() {
        let mut r = Rng::new(3);
        let mut seen = [false; 4];
        for _ in 0..1000 {
            let v = r.range_u32(2, 5);
            assert!((2..=5).contains(&v));
            seen[(v - 2) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "nem todos os valores saíram");
        assert_eq!(r.range_u32(9, 9), 9);
        assert_eq!(r.range_u32(9, 3), 9);
        // Faixa inteira de u32: o span não pode estourar.
        r.range_u32(0, u32::MAX);
    }
}
