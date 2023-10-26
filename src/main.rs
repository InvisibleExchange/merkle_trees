use std::{collections::HashMap, str::FromStr, time::Instant};

use invisible_backend::{utils::tree_utils::get_zero_hash, Tree};
use num_bigint::BigUint;
use num_traits::Zero;
use starknet_crypto::FieldElement;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tree = Tree::new(32, 0);

    let mut updated_hashes = HashMap::new();
    for i in (0..1000).into_iter().step_by(4) {
        updated_hashes.insert(i, i.to_string());
    }

    let mut preimage = serde_json::Map::new();

    let now = Instant::now();
    tree.batch_transition_updates(&updated_hashes, &mut preimage);

    println!("time to create updated_hashes: {:?}", now.elapsed());

    // println!("root: {:?}", tree.root);

    Ok(())
}
