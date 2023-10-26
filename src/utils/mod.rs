use std::str::FromStr;

use num_bigint::BigUint;
use starknet_crypto::FieldElement;

pub mod parallelization;
pub mod state_tansitions;
pub mod storage;
pub mod tree_utils;

pub fn pedersen(a: &BigUint, b: &BigUint) -> BigUint {
    let a = FieldElement::from_str(&a.to_string()).unwrap();
    let b = FieldElement::from_str(&b.to_string()).unwrap();

    let hash = starknet_crypto::pedersen_hash(&a, &b);

    return BigUint::from_str(&hash.to_string()).unwrap();
}
