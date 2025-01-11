// abstractions for numerical types that make interacting with the VM much simpler

pub trait Numerical : Copy + Clone + PartialEq + Ord {
    const BYTE_COUNT : usize;

    fn from_be(self) -> Self; // flip the endianness if we're on an LE platform

    fn to_be(self) -> Self { // flip the endianness back (this is actually exactly the same thing as from_be but the name adds clarity)
        self.from_be()
    }

    fn naive_u64(self) -> u64;

    fn from_naive_u64(v : u64) -> Self;
}


impl Numerical for u64 {
    const BYTE_COUNT : usize = 8;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        self
    }

    fn from_naive_u64(v : u64) -> Self {
        v
    }
}

impl Numerical for u32 {
    const BYTE_COUNT : usize = 4;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for u16 {
    const BYTE_COUNT : usize = 2;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for u8 {
    const BYTE_COUNT : usize = 1;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for i64 {
    const BYTE_COUNT : usize = 8;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for i32 {
    const BYTE_COUNT : usize = 4;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for i16 {
    const BYTE_COUNT : usize = 2;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}

impl Numerical for i8 {
    const BYTE_COUNT : usize = 1;

    fn from_be(self) -> Self {
        Self::from_be(self)
    }

    fn naive_u64(self) -> u64 { // NAIVELY cast this to a u64. this means that negative numbers will suddenly be absurdly large.
        let mut sp64 = [0u8; 8];
        let mbytes = self.to_be_bytes();
        for i in 0..Self::BYTE_COUNT {
            sp64[i + 7 - Self::BYTE_COUNT] = mbytes[i];
        }
        u64::from_be_bytes(sp64)
    }

    fn from_naive_u64(v : u64) -> Self {
        let bytes = v.to_be_bytes();
        Self::from_be_bytes(bytes[8 - Self::BYTE_COUNT..].try_into().unwrap())
    }
}
