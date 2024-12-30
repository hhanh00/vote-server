use anyhow::Result;
use zcash_vote::{ballot::Ballot, Election};

pub fn validate(
    election: &Election,
    input: &str,
) -> Result<Ballot> {
    let ballot: Ballot = serde_json::from_str(input)?;
    // let domain = orchard::pob::domain(election.name.as_bytes());
    // let result = validate_proof(&input, domain, &election)?;
    // Ok(result)
    Ok(ballot)
}
