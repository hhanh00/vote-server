use anyhow::Result;
use zcash_vote::{proof::ValidationResult, validate_proof, Election};

pub fn validate(
    election: &Election,
    input: &[u8],
) -> Result<ValidationResult> {
    let domain = orchard::pob::domain(election.name.as_bytes());
    let result = validate_proof(&input, domain, &election)?;
    Ok(result)
}


