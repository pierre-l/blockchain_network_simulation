mod pow;
mod miner;

use std::u8::MAX as U8_MAX;
use std::sync::Arc;
use blockchain::pow::{Hash, Nonce};
pub use blockchain::miner::mine;
pub use blockchain::pow::Difficulty;

pub struct Block{
    node_id: u8,
    nonce: Nonce,
    hash: Hash,
    previous_block_hash: Hash,
}

impl Block{
    pub fn new(node_id: u8, nonce: Nonce, previous_block_hash: Hash) -> Block {
        let hash = Hash::new(node_id, &nonce);
        Block{
            node_id,
            nonce,
            hash,
            previous_block_hash,
        }
    }

    pub fn genesis_block() -> Block {
        let nonce = Nonce::new();
        let genesis_node_id = U8_MAX;
        let hash = Hash::new(genesis_node_id, &nonce);
        Block{
            node_id: genesis_node_id,
            nonce,
            previous_block_hash: hash.clone(),
            hash,
        }
    }

    pub fn is_valid(&self, difficulty: &Arc<Difficulty>) -> bool {
        if self.hash.less_than(difficulty) {
            let hash = Hash::new(self.node_id, &self.nonce);

            hash.eq(&self.hash)
        } else {
            false
        }
    }

    pub fn hash(&self) -> &Hash{
        &self.hash
    }
}

pub struct Chain{
    head: Block,
    tail: Option<Arc<Chain>>,
    difficulty: Arc<Difficulty>,
    height: usize,
}

impl Chain{
    pub fn init_new(difficulty: Difficulty) -> Chain{
        Chain{
            head: Block::genesis_block(),
            tail: None,
            difficulty: Arc::new(difficulty),
            height: 0,
        }
    }

    pub fn expand(chain: &Arc<Chain>, block: Block) -> Result<Arc<Chain>, ()> {
        if Chain::hashes_match(&chain, &block)
            && block.is_valid(&chain.difficulty) {
            let new_chain = Chain {
                head: block,
                difficulty: chain.difficulty.clone(),
                height: chain.height + 1,
                tail: Some(chain.clone()),
            };

            Ok(Arc::new(new_chain))
        } else {
            Err(())
        }
    }

    pub fn head(&self) -> &Block {
        &self.head
    }

    pub fn height(&self) -> &usize {
        &self.height
    }

    fn hashes_match(chain: &Arc<Chain>, block: &Block) -> bool {
        chain.head.hash.eq(&block.previous_block_hash)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_and_expand_a_chain() {
        let mut difficulty = Difficulty::min_difficulty();
        difficulty.increase();

        let chain = Chain::init_new(difficulty);
        let mut chain = Arc::new(chain);

        let node_id = 1;
        let mut nonce = Nonce::new();

        while {
            nonce.increment();
            let block = Block::new(node_id, nonce.clone(), chain.head().hash().clone());

            let new_chain = match Chain::expand(&chain, block){
                Ok(chain) => {
                    Some(chain)
                },
                Err(()) => {
                    None
                }
            };

            if let Some(new_chain) = new_chain {
                chain = new_chain;
            }

            chain.height < 5
        } {}
    }
}