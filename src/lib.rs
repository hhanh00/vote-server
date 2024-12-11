use std::collections::HashMap;

use elections::init_elections;
use zcash_vote::Election;

pub mod elections;
pub mod validate;
pub mod db;

#[macro_export]
macro_rules! execute {
    ($block:block) => {
        {
            let res = || -> Result<_> {
                $block
            };
            res().map_err(|e| Custom(Status::InternalServerError, e.to_string()))
        }
    };
}

lazy_static::lazy_static! {
    pub static ref ELECTIONS: HashMap<u32, Election> = init_elections().unwrap();
}
