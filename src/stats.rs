use std::ops::{Add, AddAssign};

#[derive(Debug, Default)]
pub struct Accumulator {
    pub file_count_found: u64,
    pub byte_count_found: u64,
    pub file_count_copied: u64,
    pub byte_count_copied: u64,
    pub file_count_skipped: u64,
    pub byte_count_skipped: u64,
}

impl Accumulator {
    #[inline(always)]
    pub fn found(files: u64, bytes: u64) -> Self {
        Self {
            file_count_found: files,
            byte_count_found: bytes,
            ..Default::default()
        }
    }

    #[inline(always)]
    pub fn copies(files: u64, bytes: u64) -> Self {
        Self {
            file_count_copied: files,
            byte_count_copied: bytes,
            ..Default::default()
        }
    }

    pub fn skips(files: u64, bytes: u64) -> Self {
        Self { file_count_skipped: files, byte_count_skipped: bytes, ..Default::default() }
    }
}

impl Add for Accumulator {
    type Output = Accumulator;

    fn add(self, rhs: Self) -> Self::Output {
        Accumulator {
            file_count_found: self.file_count_found + rhs.file_count_found,
            byte_count_found: self.byte_count_found + rhs.byte_count_found,
            file_count_copied: self.file_count_copied + rhs.file_count_copied,
            byte_count_copied: self.byte_count_copied + rhs.byte_count_copied,
            file_count_skipped: self.file_count_skipped + rhs.file_count_skipped,
            byte_count_skipped: self.byte_count_skipped + rhs.byte_count_skipped,
        }
    }
}

impl AddAssign for Accumulator {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.file_count_found += rhs.file_count_found;
        self.byte_count_found += rhs.byte_count_found;
        self.file_count_copied += rhs.file_count_copied;
        self.byte_count_copied += rhs.byte_count_copied;
        self.file_count_skipped += rhs.file_count_skipped;
        self.byte_count_skipped += rhs.byte_count_skipped;
    }
}
