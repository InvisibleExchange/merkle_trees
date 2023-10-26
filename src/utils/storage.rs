use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    str::FromStr,
};

use crate::Tree;

pub fn _store_to_disk_inner(
    leaf_nodes: &Vec<String>,
    inner_nodes: &Vec<Vec<String>>,
    root: &String,
    depth: u32,
    tree_index: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let str: String = "./storage/merkle_trees/state_tree/".to_string() + &tree_index.to_string();

    let path = Path::new(&str);
    if !Path::new("./storage/merkle_trees/state_tree/").exists() {
        fs::create_dir("./storage/merkle_trees/state_tree/")?;
    }

    let mut file: File = File::create(path)?;

    let leaves = leaf_nodes;
    // .iter()
    // .map(|x| ())
    // .collect::<Vec<[u8; 32]>>();

    let inner_nodes = inner_nodes
        .iter()
        .map(|x| x.iter().map(|y| y.to_string()).collect::<Vec<String>>())
        .collect::<Vec<Vec<String>>>();

    let encoded: Vec<u8> =
        bincode::serialize(&(leaves, inner_nodes, root.to_string(), depth)).unwrap();

    file.write_all(&encoded[..])?;

    Ok(())
}

pub fn _from_disk_inner(
    tree_index: u32,
    depth: u32,
    shift: u32,
) -> Result<Tree, Box<dyn std::error::Error>> {
    let str = "./storage/merkle_trees/state_tree/";
    let path_str = str.to_string() + &tree_index.to_string();
    let path = Path::new(&path_str);

    let open_res = File::open(path).ok();
    if open_res.is_none() {
        if Path::new(&str).exists() {
            File::create(path)?;
            return Ok(Tree::new(depth, shift));
        } else {
            fs::create_dir(&str)?;
            File::create(path)?;
            return Ok(Tree::new(depth, shift));
        }
    };

    let mut file: File = open_res.unwrap();
    let mut buf: Vec<u8> = Vec::new();

    file.read_to_end(&mut buf)?;

    let decoded: (Vec<String>, Vec<Vec<String>>, String, u32) = bincode::deserialize(&buf[..])?;

    let leaves = decoded.0;
    // .iter()
    // .map(|x| String::from_bytes_be(x).unwrap())
    // .collect();
    let inner_nodes = decoded
        .1
        .iter()
        .map(|x| {
            x.iter()
                .map(|y| String::from_str(y.as_str()).unwrap())
                .collect::<Vec<String>>()
        })
        .collect::<Vec<Vec<String>>>();

    Ok(Tree {
        leaf_nodes: leaves,
        inner_nodes,
        root: String::from_str(&decoded.2.as_str()).unwrap(),
        depth: decoded.3,
        shift,
    })
}
