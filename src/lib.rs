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
