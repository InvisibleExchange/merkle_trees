pub mod utils;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use num_bigint::BigUint;
use num_traits::Zero;
use parking_lot::Mutex;
use serde_json::{Map, Value};
use utils::{
    parallelization::{split_and_run_first_row, split_and_run_next_row},
    // storage::{_from_disk_inner, _store_to_disk_inner},
    tree_utils::{idx_to_binary_pos, inner_from_leaf_nodes_vr, pad_leaf_nodes_vr, proof_pos},
};

use crate::utils::tree_utils::get_zero_hash;

#[derive(Debug, Clone)]
pub struct Tree {
    pub leaf_nodes: Vec<BigUint>,
    pub inner_nodes: Vec<Vec<BigUint>>,
    pub depth: u32,
    pub root: BigUint,
    pub shift: u32, // in case of a root tree we can start at a different depth
}

impl Tree {
    pub fn new(depth: u32, shift: u32) -> Tree {
        let leaf_nodes: Vec<BigUint> = Vec::new();
        let mut inner_nodes: Vec<Vec<BigUint>> = Vec::new();
        let root = get_zero_hash(depth, shift);

        for _ in 0..depth {
            let empty_vec: Vec<BigUint> = Vec::new();
            inner_nodes.push(empty_vec);
        }

        return Tree {
            leaf_nodes,
            inner_nodes,
            depth,
            root,
            shift,
        };
    }

    // -----------------------------------------------------------------
    // Optimized parallel transition from one tx_batch to another
    // Updates the tree with a batch of updates and generates the preimage multi update proofs
    ///
    /// # Arguments
    ///
    /// * `updated_hashes` - The hashmap of all the leaf nodes that need to be updated {idx: new_hash}
    /// * `preimage` - the json_map to be filed with the preimage hashes
    pub fn batch_transition_updates(
        &mut self,
        updated_hashes: &HashMap<u64, BigUint>,
        preimage: &mut Map<String, Value>,
    ) {
        //

        if updated_hashes.len() == 0 {
            return;
        }

        let tree_depth = self.depth;

        let tree_mutex = Arc::new(Mutex::new(self));
        let preimage_mutex = Arc::new(Mutex::new(preimage));

        let mut next_row = split_and_run_first_row(&tree_mutex, &preimage_mutex, updated_hashes, 0);

        for i in 1..tree_depth as usize {
            next_row = split_and_run_next_row(&tree_mutex, &preimage_mutex, &next_row, i, 0);
        }

        let mut tree = tree_mutex.lock();
        tree.root = tree.inner_nodes[tree_depth as usize - 1][0].clone();
        drop(tree);
    }

    // -----------------------------------------------------------------
    // HELPERS

    fn update_leaf_node(&mut self, leaf_hash: &BigUint, idx: u64) {
        assert!(idx < 2_u64.pow(self.depth), "idx is greater than tree size");

        if self.leaf_nodes.len() > idx as usize {
            self.leaf_nodes[idx as usize] = leaf_hash.clone();
        } else {
            let len_diff = idx as usize - self.leaf_nodes.len();

            for _ in 0..len_diff {
                self.leaf_nodes.push(BigUint::zero());
            }

            self.leaf_nodes.push(leaf_hash.clone())
        }
    }

    fn update_inner_node(&mut self, i: u32, j: u64, value: BigUint) {
        assert!(i <= self.depth, "i is greater than depth");
        assert!(j < 2_u64.pow(self.depth - i), "j is greater than 2^i");

        if self.inner_nodes.get(i as usize - 1).unwrap().len() > j as usize {
            self.inner_nodes[i as usize - 1][j as usize] = value;
        } else {
            let len_diff = j as usize - self.inner_nodes[i as usize - 1].len();

            for _ in 0..len_diff {
                self.inner_nodes[i as usize - 1].push(get_zero_hash(i, self.shift));
            }

            self.inner_nodes[i as usize - 1].push(value);
        }
    }

    fn nth_leaf_node(&self, n: u64) -> BigUint {
        assert!(n < 2_u64.pow(self.depth), "n is bigger than tree size");

        if self.leaf_nodes.get(n as usize).is_some() {
            return self.leaf_nodes[n as usize].clone();
        } else {
            return get_zero_hash(0, self.shift);
        }
    }

    fn ith_inner_node(&self, i: u32, j: u64) -> BigUint {
        // ? Checks if the inner note at that spot exists, else it returns the zero hash

        assert!(i <= self.depth, "i is greater than depth");
        assert!(j < 2_u64.pow(self.depth - i), "j is greater than 2^i");

        if self.inner_nodes.get(i as usize - 1).is_some()
            && self.inner_nodes[i as usize - 1].get(j as usize).is_some()
        {
            let res = self.inner_nodes[i as usize - 1][j as usize].clone();
            return res;
        } else {
            let zero_hash = get_zero_hash(i, self.shift);
            return zero_hash;
        }
    }

    // I/O Operations --------------------------------------------------

    // /// Stores the tree to disk. Tree index is the index of the tree in the storage folder.
    // pub fn store_to_disk(&self, tree_index: u32) -> Result<(), Box<dyn Error>> {
    //     _store_to_disk_inner(
    //         &self.leaf_nodes,
    //         &self.inner_nodes,
    //         &self.root,
    //         self.depth,
    //         tree_index,
    //     )
    // }

    // /// Fetches the tree stored on disk and reconstructs it.
    // pub fn from_disk(tree_index: u32, depth: u32, shift: u32) -> Result<Tree, Box<dyn Error>> {
    //     _from_disk_inner(tree_index, depth, shift)
    // }

    // -----------------------------------------------------------------

    /// Get the merkle proof for a leaf node.
    pub fn get_proof(&self, leaf_idx: u64) -> (Vec<BigUint>, Vec<i8>) {
        let proof_binary_pos = idx_to_binary_pos(leaf_idx, self.depth as usize);

        let proof_pos = proof_pos(leaf_idx, self.depth as usize);

        let mut proof: Vec<BigUint> = Vec::new();
        proof.push(self.nth_leaf_node(proof_pos[0]));

        for i in 1..self.depth {
            let proof_val = self.ith_inner_node(i, proof_pos[i as usize] as u64);

            proof.push(proof_val);
        }

        return (proof, proof_binary_pos);
    }

    // -----------------------------------------------------------------

    /// Testing function that hashes the tree from the leaf nodes and checks if the root is correct (non-optimized)
    pub fn verify_root(&self) -> bool {
        let leaf_nodes = pad_leaf_nodes_vr(&self.leaf_nodes, self.depth as usize, BigUint::zero());

        let inner_nodes: Vec<Vec<BigUint>> =
            inner_from_leaf_nodes_vr(self.depth as usize, &leaf_nodes);
        let root = inner_nodes[0][0].clone();

        return self.root == root;
    }
}

//

//

//

#[cfg(test)]
mod tests {

    #[test]
    fn test1() -> Result<(), Box<dyn std::error::Error>> {
        // let mut tree = Tree::new(32, 0);

        // let mut updated_hashes = HashMap::new();
        // for i in (0..100_000).into_iter().step_by(4) {
        //     updated_hashes.insert(i, BigUint::from(i));
        // }

        // let mut preimage = serde_json::Map::new();

        // tree.batch_transition_updates(&updated_hashes, &mut preimage);

        // println!("root: {:?}", tree.root);

        Ok(())
    }
}
