use std::str::FromStr;

use starknet_crypto::FieldElement;

pub mod parallelization;
pub mod state_tansitions;
pub mod storage;
pub mod tree_utils;

pub fn pedersen(a: &String, b: &String) -> String {
    let a = FieldElement::from_str(&a).unwrap();
    let b = FieldElement::from_str(&b).unwrap();

    let hash = starknet_crypto::pedersen_hash(&a, &b);

    return hash.to_string();
}
