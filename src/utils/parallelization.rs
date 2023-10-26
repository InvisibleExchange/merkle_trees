use std::{collections::HashMap, sync::Arc, time::Instant};

use parking_lot::Mutex;
use serde_json::{Map, Value};

use crate::{
    utils::{pedersen, tree_utils::get_zero_hash},
    Tree,
};

// * =================================================================================================================
// * HELPER FUNCTION FOR PARALLEL UPDATES

const STRIDE: usize = 250; // Must be even

pub fn split_and_run_first_row(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    updated_hashes: &HashMap<u64, String>,
    n: usize,
) -> HashMap<u64, String> {
    let next_row_proofs: HashMap<u64, String> = HashMap::new();
    let next_row_proofs_mutex = Arc::new(Mutex::new(next_row_proofs));

    split_and_run_first_row_inner(
        tree_mutex,
        preimage_mutex,
        updated_hashes,
        &next_row_proofs_mutex,
        n,
    );

    let res = next_row_proofs_mutex.lock();
    return res.to_owned();
}

fn split_and_run_first_row_inner(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    updated_hashes: &HashMap<u64, String>,
    next_row: &Arc<Mutex<HashMap<u64, String>>>,
    n: usize,
) {
    // ? n counts how deep in the recursion loop we are
    // ? at each iteration we take four elements from the hashmap and update the tree

    let elems: Vec<(&u64, &String)> = updated_hashes
        .iter()
        .skip(n * STRIDE)
        .take(STRIDE)
        .collect();

    // ? As long as there are elements in the map (elems.len() > 0) we keep splitting
    // ? Pass the rest forward recursively to run in parallel
    if elems.len() > 0 {
        rayon::join(
            || {
                let next_row_indexes =
                    build_first_row(tree_mutex, preimage_mutex, elems, updated_hashes);
                let mut next_proofs = next_row.lock();
                for (i, prev_res) in next_row_indexes {
                    next_proofs.insert(i, prev_res);
                }
                drop(next_proofs);
            },
            || {
                split_and_run_first_row_inner(
                    tree_mutex,
                    preimage_mutex,
                    updated_hashes,
                    next_row,
                    n + 1,
                )
            },
        );
    }
}

// ------------------------------

pub fn split_and_run_next_row(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    updated_hashes: &HashMap<u64, String>,
    row_depth: usize,
    n: usize,
) -> HashMap<u64, String> {
    let next_row_proofs: HashMap<u64, String> = HashMap::new();
    let next_row_proofs_mutex = Arc::new(Mutex::new(next_row_proofs));

    split_and_run_next_row_inner(
        tree_mutex,
        preimage_mutex,
        updated_hashes,
        &next_row_proofs_mutex,
        row_depth,
        n,
    );

    let res = next_row_proofs_mutex.lock();
    return res.to_owned();
}

fn split_and_run_next_row_inner(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    updated_hashes: &HashMap<u64, String>,
    next_row: &Arc<Mutex<HashMap<u64, String>>>,
    row_depth: usize,
    n: usize,
) {
    // ? n counts how deep in the recursion loop we are
    // ? at each iteration we take four elements from the hashmap and update the tree

    let elems: Vec<(&u64, &String)> = updated_hashes
        .iter()
        .skip(n * STRIDE)
        .take(STRIDE)
        .collect();

    // ? As long as there are elements in the map (elems.len() > 0) we keep splitting
    // ? Pass the rest forward recursively to run in parallel
    if elems.len() > 0 {
        rayon::join(
            || {
                let next_row_indexes =
                    build_next_row(tree_mutex, preimage_mutex, elems, updated_hashes, row_depth);
                let mut next_proofs = next_row.lock();
                for (i, prev_res) in next_row_indexes {
                    next_proofs.insert(i, prev_res);
                }
                drop(next_proofs);
            },
            || {
                split_and_run_next_row_inner(
                    tree_mutex,
                    preimage_mutex,
                    updated_hashes,
                    next_row,
                    row_depth,
                    n + 1,
                )
            },
        );
    }
}

// ------------------------------

fn build_first_row(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    entries: Vec<(&u64, &String)>, // 4 entries taken from the hashmap to be updated in parallel
    hashes: &HashMap<u64, String>, // the whole hashmap
) -> Vec<(u64, String)> {
    // next row stores the indexes of the next row that need to be updated
    // (and the previous result hashes for the init state preimage)
    let mut next_row: Vec<(u64, String)> = Vec::new();

    for (idx, hash) in entries.iter() {
        // ! Left child
        if *idx % 2 == 0 {
            //? If the right child exists, hash them together in the next loop
            if hashes.get(&(*idx + 1)).is_some() {
                continue;
            }
            //? If the right child doesn't exist (wasn't updated), hash the left child with the previous value in the state tree
            else {
                // ? Get the previous values in the state tree
                let tree = tree_mutex.lock();
                let init_left_hash = tree.nth_leaf_node(**idx);
                let right_hash = &tree.nth_leaf_node(*idx + 1);
                drop(tree);

                // ? Hash the left child with the right child
                let new_hash = pedersen(&hash, &right_hash);

                // ? Use the new_hash to update the merkle tree
                let mut tree = tree_mutex.lock();
                let prev_res_hash = tree.ith_inner_node(1, *idx / 2);
                tree.update_inner_node(1, *idx / 2, new_hash.clone());
                drop(tree);

                next_row.push((*idx / 2, prev_res_hash.clone()));

                // * Preimages -----------------------------------------------------------------------------------------------

                // ? Insert the new hash info into the preimage
                let mut preimage = preimage_mutex.lock();

                if !preimage.contains_key(&prev_res_hash.to_string()) {
                    preimage.insert(
                        prev_res_hash.to_string(),
                        serde_json::to_value([init_left_hash.to_string(), right_hash.to_string()])
                            .unwrap(),
                    );
                }

                preimage.insert(
                    new_hash.to_string(),
                    serde_json::to_value([hash.to_string(), right_hash.to_string()]).unwrap(),
                );
                drop(preimage);

                // * Preimages -----------------------------------------------------------------------------------------------

                // ? update the leaf node with the hash
                let mut tree = tree_mutex.lock();
                tree.update_leaf_node(hash, **idx);
                drop(tree);
            }
        }
        // ! Right child
        else {
            // ? get the left child hash
            let left_hash: String;
            let prev_left_hash: String;
            let prev_right_hash: String;
            if hashes.get(&(*idx - 1)).is_some() {
                // ? If the left child exists, hash them together
                left_hash = hashes.get(&(*idx - 1)).unwrap().clone();
                let mut tree = tree_mutex.lock();
                prev_left_hash = tree.nth_leaf_node(*idx - 1);
                prev_right_hash = tree.nth_leaf_node(**idx);

                // ? Update the nodes in the tree with the hashes
                tree.update_leaf_node(&left_hash, **idx - 1);
                tree.update_leaf_node(hash, **idx);

                drop(tree);
            } else {
                //? If the left child doesn't exist, hash the right child with the previous value in the state tree
                let mut tree = tree_mutex.lock();
                left_hash = tree.nth_leaf_node(*idx - 1);
                prev_left_hash = tree.nth_leaf_node(*idx - 1);
                prev_right_hash = tree.nth_leaf_node(**idx);

                // ? Update the nodes in the tree with the hashes
                tree.update_leaf_node(hash, **idx);
                drop(tree);
            };

            // ? Hash the left child with the right child
            let new_hash = pedersen(&left_hash, &hash);

            // ? Use the new_hash to update the merkle tree
            let mut tree = tree_mutex.lock();
            let prev_res_hash = tree.ith_inner_node(1, *idx / 2);
            tree.update_inner_node(1, *idx / 2, new_hash.clone());
            drop(tree);
            next_row.push((*idx / 2, prev_res_hash.clone()));

            // * Preimages -----------------------------------------------------------------------------------------------

            // ? Insert the new hash info into the preimage
            let mut preimage = preimage_mutex.lock();

            if !preimage.contains_key(&prev_res_hash.to_string()) {
                preimage.insert(
                    prev_res_hash.to_string(),
                    serde_json::to_value([prev_left_hash.to_string(), prev_right_hash.to_string()])
                        .unwrap(),
                );
            }

            preimage.insert(
                new_hash.to_string(),
                serde_json::to_value([left_hash.to_string(), hash.to_string()]).unwrap(),
            );
            drop(preimage);

            // * Preimages -----------------------------------------------------------------------------------------------
        }
    }

    return next_row;
}

fn build_next_row(
    tree_mutex: &Arc<Mutex<&mut Tree>>,
    preimage_mutex: &Arc<Mutex<&mut Map<String, Value>>>,
    entries: Vec<(&u64, &String)>, // 4 entries taken from the hashmap to be updated in parallel
    hashes: &HashMap<u64, String>, // the whole hashmap
    row_depth: usize,
) -> Vec<(u64, String)> {
    // next row stores the indexes of the next row that need to be updated
    // (and the previous result hashes for the init state preimage)
    let mut next_row: Vec<(u64, String)> = Vec::new();

    for (idx, prev_res) in entries.iter() {
        // ! Left child
        if *idx % 2 == 0 {
            //? If the right child exists, hash them together in the next loop
            if hashes.get(&(*idx + 1)).is_some() {
                continue;
            }
            //? If the right child doesn't exist (hasn't been updated), hash the left child with the previous value in the state tree
            else {
                // ? Get the previous values from the state tree
                let tree = tree_mutex.lock();
                let hash = &tree.ith_inner_node(row_depth as u32, **idx);
                let right_hash = &tree.ith_inner_node(row_depth as u32, *idx + 1);
                drop(tree);

                // ? Hash the left child with the right child
                let new_hash = pedersen(hash, right_hash);

                // ? Use the new_hash to update the merkle tree
                let mut tree = tree_mutex.lock();
                let prev_res_hash = tree.ith_inner_node(row_depth as u32 + 1, *idx / 2);
                tree.update_inner_node(row_depth as u32 + 1, *idx / 2, new_hash.clone());
                drop(tree);
                next_row.push((*idx / 2, prev_res_hash.clone()));

                // * Preimages -----------------------------------------------------------------------------------------------

                // ? Insert the new hash info into the preimage
                let mut preimage = preimage_mutex.lock();

                // ? Previous batch state preimage
                if !preimage.contains_key(&prev_res_hash.to_string()) {
                    preimage.insert(
                        prev_res_hash.to_string(),
                        serde_json::to_value([prev_res.to_string(), right_hash.to_string()])
                            .unwrap(),
                    );
                }

                // ? Current batch state preimage
                preimage.insert(
                    new_hash.to_string(),
                    serde_json::to_value([hash.to_string(), right_hash.to_string()]).unwrap(),
                );
                drop(preimage);

                // * Preimages -----------------------------------------------------------------------------------------------
            }
        }
        // ! Right child
        else {
            // ? Get the left and right hashes
            let tree = tree_mutex.lock();

            let hash = &tree.ith_inner_node(row_depth as u32, **idx);
            let left_hash = &tree.ith_inner_node(row_depth as u32, *idx - 1);
            let prev_left_hash: String;
            if let Some(prev_left) = hashes.get(&(*idx - 1)) {
                prev_left_hash = prev_left.clone();
            } else {
                prev_left_hash = left_hash.clone();
            }
            let prev_right_hash = *prev_res;

            drop(tree);

            // ? Hash the left child with the right child
            let new_hash = pedersen(&left_hash, &hash);

            // ? Use the new_hash to update the merkle tree
            let mut tree = tree_mutex.lock();
            let prev_res_hash = tree.ith_inner_node(row_depth as u32 + 1, *idx / 2);
            tree.update_inner_node(row_depth as u32 + 1, *idx / 2, new_hash.clone());
            drop(tree);

            next_row.push((*idx / 2, prev_res_hash.clone()));

            // * Preimages -----------------------------------------------------------------------------------------------

            // ? Insert the new hash info into the preimage
            let mut preimage = preimage_mutex.lock();

            // ? Previous batch state preimage
            if !preimage.contains_key(&prev_res_hash.to_string()) {
                preimage.insert(
                    prev_res_hash.to_string(),
                    serde_json::to_value([prev_left_hash.to_string(), prev_right_hash.to_string()])
                        .unwrap(),
                );
            }

            // ? Current batch state preimage
            preimage.insert(
                new_hash.to_string(),
                serde_json::to_value([left_hash.to_string(), hash.to_string()]).unwrap(),
            );
            drop(preimage);

            // * Preimages -----------------------------------------------------------------------------------------------
        }
    }

    return next_row;
}

pub fn build_tree(depth: u32, leaf_nodes: &Vec<String>, shift: u32) -> String {
    let inner_nodes: Vec<Vec<String>> = inner_from_leaf_nodes(depth as usize, leaf_nodes, shift);
    let root = inner_nodes[0][0].clone();

    return root;
}

fn inner_from_leaf_nodes(depth: usize, leaf_nodes: &Vec<String>, shift: u32) -> Vec<Vec<String>> {
    let mut tree: Vec<Vec<String>> = Vec::new();

    let first_row = leaf_nodes;

    let len = leaf_nodes.len();
    let new_len = if len % 2 == 0 { len / 2 } else { len / 2 + 1 };
    let mut hashes: Vec<String> = vec!["0".to_string(); new_len];
    let hashes_mutex = Arc::new(Mutex::new(&mut hashes));
    hash_tree_level(&hashes_mutex, &first_row, 0, 0, shift);
    tree.push(hashes);

    for i in 1..depth {
        let len = &tree[i - 1].len();
        let new_len = if len % 2 == 0 { len / 2 } else { len / 2 + 1 };
        let mut hashes: Vec<String> = vec!["0".to_string(); new_len];
        let hashes_mutex = Arc::new(Mutex::new(&mut hashes));
        hash_tree_level(&hashes_mutex, &tree[i - 1], i, 0, shift);
        tree.push(hashes);
    }

    tree.reverse();
    return tree;
}

fn hash_tree_level(
    next_row: &Arc<Mutex<&mut Vec<String>>>,
    leaf_nodes: &Vec<String>,
    i: usize,
    n: usize,
    shift: u32,
) {
    let inp_array = leaf_nodes
        .iter()
        .skip(n * STRIDE)
        .take(STRIDE)
        .collect::<Vec<&String>>();

    // println!("inp_array: {:?}", inp_array);

    if inp_array.len() > 0 {
        rayon::join(
            || {
                let next_row_hashes = pairwise_hash(&inp_array, i, shift);
                let mut next_hashes = next_row.lock();

                let hashes_len = next_hashes.len();
                if hashes_len < (n * STRIDE) / 2 + STRIDE / 2 || next_row_hashes.len() < STRIDE / 2
                {
                    next_hashes.as_mut_slice()[(n * STRIDE) / 2..]
                        .clone_from_slice(&next_row_hashes);
                } else {
                    next_hashes.as_mut_slice()[(n * STRIDE) / 2..(n * STRIDE) / 2 + STRIDE / 2]
                        .clone_from_slice(&next_row_hashes);
                }

                drop(next_hashes);
            },
            || hash_tree_level(next_row, leaf_nodes, i, n + 1, shift),
        );
    }
}

pub fn pairwise_hash(array: &Vec<&String>, i: usize, shift: u32) -> Vec<String> {
    // This should be an array of STRIDE length

    let mut hashes: Vec<String> = Vec::new();
    for j in (0..array.len() - 1).step_by(2) {
        let hash = pedersen(&array[j], &array[j + 1]);
        hashes.push(hash);
    }

    if array.len() % 2 == 1 {
        hashes.push(pedersen(
            &array[array.len() - 1],
            &get_zero_hash(i as u32, shift),
        ));
    }

    return hashes;
}
