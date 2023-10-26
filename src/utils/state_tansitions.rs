// use rayon::prelude::{IntoParallelIterator, ParallelIterator};
// use serde_json::{Map, Value};
// use starknet_crypto::FieldElement;
// use std::collections::HashMap;

// use std::error::Error;
// use std::result::Result;

// use crate::Tree;

// /// This functions fetches all the merkle trees from storage and updates them and stores the updated trees back to storage.
// /// This allows the main merkle tree to be broken up into smaller trees that can be updated in parallel
// /// and requires significantly less memory to update.
// ///
// /// # Arguments
// ///
// /// * `updated_state_hashes` - The hashmap of all the leaf nodes that need to be updated {idx: new_hash}
// /// * `total_depth` - the total depth of the main merkle tree (this can be spilt up into shallower trees of depth `partition_size_exponent`)
// pub fn update_trees(
//     updated_state_hashes: HashMap<u64, FieldElement>,
//     total_depth: u32,
//     partition_size_exponent: u32,
// ) -> Result<(FieldElement, FieldElement, Map<FieldElement, Value>), Box<dyn Error>> {
//     // * UPDATE SPOT TREES  -------------------------------------------------------------------------------------
//     let mut updated_root_hashes: HashMap<u64, FieldElement> = HashMap::new(); // the new roots of all tree partitions

//     let mut preimage_json: Map<FieldElement, Value> = Map::new();

//     let partitioned_hashes = split_hashmap(
//         updated_state_hashes,
//         2_usize.pow(partition_size_exponent) as usize,
//     );

//     // ? Loop over all partitions and update the trees
//     for (partition_index, partition) in partitioned_hashes {
//         if partition.is_empty() {
//             continue;
//         }

//         let (_, new_root) = tree_partition_update(
//             partition,
//             &mut preimage_json,
//             partition_index as u32,
//             total_depth,
//             partition_size_exponent,
//         )?;

//         updated_root_hashes.insert(partition_index as u64, new_root);
//     }

//     // ? use the newly generated roots to update the state tree
//     let (prev_spot_root, new_spot_root) = tree_partition_update(
//         updated_root_hashes,
//         &mut preimage_json,
//         u32::MAX,
//         total_depth,
//         partition_size_exponent,
//     )?;

//     Ok((prev_spot_root, new_spot_root, preimage_json))
// }

// fn tree_partition_update(
//     updated_state_hashes: HashMap<u64, FieldElement>,
//     preimage_json: &mut Map<FieldElement, Value>,
//     tree_index: u32,
//     total_depth: u32,
//     partition_size_exponent: u32,
// ) -> Result<(FieldElement, FieldElement), Box<dyn Error>> {
//     let shift = if tree_index == u32::MAX {
//         partition_size_exponent
//     } else {
//         0
//     };
//     let depth = if tree_index == u32::MAX {
//         total_depth - partition_size_exponent
//     } else {
//         partition_size_exponent
//     };

//     let mut batch_init_tree = Tree::from_disk(tree_index, depth, shift)?;

//     let prev_root = batch_init_tree.root.clone();

//     // ? Store the current tree to disk as a backup
//     batch_init_tree.store_to_disk(tree_index)?;

//     batch_init_tree.batch_transition_updates(&updated_state_hashes, preimage_json);

//     let new_root = batch_init_tree.root.clone();

//     Ok((prev_root, new_root))
// }

// // * ================================================================================

// /// Splits a hashmap into submaps of size `chunk_size`.
// pub fn split_hashmap(
//     hashmap: HashMap<u64, FieldElement>,
//     chunk_size: usize,
// ) -> Vec<(usize, HashMap<u64, FieldElement>)> {
//     let max_key = *hashmap.keys().max().unwrap_or(&0);
//     let num_submaps = (max_key as usize + chunk_size) / chunk_size;

//     let submaps: Vec<(usize, HashMap<u64, FieldElement>)> = (0..num_submaps)
//         .into_par_iter()
//         .map(|submap_index| {
//             let submap: HashMap<u64, FieldElement> = hashmap
//                 .iter()
//                 .filter(|(key, _)| {
//                     let submap_start = if submap_index == 0 {
//                         0
//                     } else {
//                         submap_index * chunk_size
//                     };
//                     let submap_end = (submap_index + 1) * chunk_size;
//                     **key >= submap_start as u64 && **key < submap_end as u64
//                 })
//                 .map(|(key, value)| (key % chunk_size as u64, value.clone()))
//                 .collect();

//             (submap_index, submap)
//         })
//         .collect();

//     submaps
// }
