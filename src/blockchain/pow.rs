use ring::digest::{self, Digest, SHA256, SHA256_OUTPUT_LEN};
use std::cmp::Ordering;
use std::u8::MAX as U8_MAX;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;

const DIFFICULTY_BYTES_LEN: usize = SHA256_OUTPUT_LEN;
#[derive(Clone)]
pub struct Difficulty([u8; SHA256_OUTPUT_LEN]);

impl Difficulty{
    pub fn min_difficulty() -> Difficulty{
        let array = [U8_MAX as u8; SHA256_OUTPUT_LEN];
        Difficulty(array)
    }

    pub fn increase(&mut self) {
        self.divide_inner_by_two()
    }

    fn divide_inner_by_two(&mut self){
        let mut index_to_split = 0;

        while self.0[index_to_split] == 0 {
            index_to_split += 1;
        }
        self.0[index_to_split] /= 2;

        if self.0[index_to_split] == 0 {
            let next_index = index_to_split + 1;

            self.0[next_index] = U8_MAX/2;
        }
    }
}

impl Debug for Difficulty{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        print_u8_as_hexa(&self.0, f)
    }
}

#[derive(Clone)]
pub struct Hash{
    digest: Digest,
}

impl Hash{
    pub fn new(
        node_id: u32,
        nonce: &Nonce,
        difficulty: &Difficulty,
        previous_hash: &[u8],
    ) -> Hash{
        let difficulty_bytes = difficulty.0.as_ref();
        let mut data_to_hash = [0u8;
            8 // Length of the nonce field.
                + 4 // Length of the node_id field.
                + SHA256_OUTPUT_LEN // Length of the hash.
                + DIFFICULTY_BYTES_LEN
        ];

        data_to_hash[..8].clone_from_slice(&nonce.0[..8]);

        data_to_hash[8] = ((node_id >> 24) & 0xff) as u8;
        data_to_hash[9] = ((node_id >> 16) & 0xff) as u8;
        data_to_hash[10] = ((node_id >> 8) & 0xff) as u8;
        data_to_hash[11] = (node_id & 0xff) as u8;

        let index = 12;
        data_to_hash[index..(SHA256_OUTPUT_LEN + index)].clone_from_slice(&previous_hash[..SHA256_OUTPUT_LEN]);

        let index = index + SHA256_OUTPUT_LEN;
        data_to_hash[index..(DIFFICULTY_BYTES_LEN + index)].clone_from_slice(&difficulty_bytes[..DIFFICULTY_BYTES_LEN]);

        let digest = digest::digest(&SHA256, &data_to_hash);

        Hash{
            digest,
        }
    }

    pub fn less_than(&self, difficulty: &Difficulty) -> bool {
        let hash_bytes = self.bytes();
        let difficulty_bytes = &difficulty.0;

        debug!("Candidate:  {:?}", hash_bytes);
        debug!("Difficulty: {:?}", difficulty_bytes);

        // Can't use `cmp` between these because the digest's [u8] length.
        less_than_u8(hash_bytes, difficulty_bytes)
    }

    pub fn bytes(&self) -> &[u8]{
        self.digest.as_ref()
    }
}

impl Debug for Hash{
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        print_u8_as_hexa(&self.bytes(), f)
    }
}

impl PartialEq for Hash{
    fn eq(&self, other: &Hash) -> bool {
        self.digest.as_ref().eq(other.digest.as_ref())
    }
}

fn less_than_u8(one: &[u8], other: &[u8]) -> bool{
    // Still, we assume that `one` and `other` have the same length.
    let len = one.len();
    let mut i = 0;
    let mut temp_result = Ordering::Equal;

    while i<len && temp_result==Ordering::Equal {
        temp_result = one[i].cmp(&other[i]);
        i += 1;
    }

    temp_result == Ordering::Less
}

#[derive(Clone, Debug)]
pub struct Nonce([u8; 8]);

impl Nonce{
    pub fn new() -> Nonce {
        Nonce([0u8; 8])
    }

    pub fn increment(&mut self) {
        let mut index_to_increment = self.0.len() -1;

        while self.0[index_to_increment] == U8_MAX {
            self.0[index_to_increment] = 0;
            index_to_increment -= 1;
        }
        self.0[index_to_increment] += 1;
    }
}

fn print_u8_as_hexa(bytes: &[u8], f: &mut Formatter) -> Result<(), Error> {
    let mut concatenated = String::new();
    for byte in bytes {
        let hex_byte = format!("{:x}", byte);

        if hex_byte.len() == 1 {
            concatenated += "0";
        }

        concatenated += &hex_byte;
    }
    write!(f, "{}", &concatenated)?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_difficulty_allows_any_hash() {
        let difficulty = Difficulty::min_difficulty();

        let mut nonce = Nonce::new();
        for _i in 0..100 {
            nonce.increment();
            let hash = Hash::new(1, &nonce, &difficulty, &[0u8; SHA256_OUTPUT_LEN]);
            assert_eq!(true, hash.less_than(&difficulty));
        }
    }

    #[test]
    fn can_increase_difficulty() {
        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();
        difficulty.increase();
        difficulty.increase();

        let number_of_tries = 100000;
        let mut number_of_valid_hashes = 0;
        let mut nonce = Nonce::new();
        for _i in 0..number_of_tries {
            nonce.increment();
            let hash = Hash::new(1, &nonce, &difficulty, &[0u8; SHA256_OUTPUT_LEN]);

            if hash.less_than(&difficulty) {
                number_of_valid_hashes += 1;
            }
        }

        assert!(number_of_valid_hashes < number_of_tries/7);
        assert!(number_of_valid_hashes > number_of_tries/9);
    }
}