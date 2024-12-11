use anyhow::Result;
use zcash_vote::Election;
use std::{collections::HashMap, fs};

pub fn init_elections() -> Result<HashMap<u32, Election>> {
    let mut elections = HashMap::<u32, Election>::new();

    let current_dir = std::env::current_dir()?;
    for entry in fs::read_dir(current_dir)? {
        let e = entry?;
        if let Some(name) = e.file_name().to_str() {
            if name.ends_with(".vote") {
                let election_json = fs::File::open(name)?;
                let v = serde_json::from_reader::<_, Election>(&election_json)?;
                println!("Election {} loaded", v.name);
                let id = v.id;
                elections.insert(id, v);
            }
        }
    }

    Ok(elections)
}
