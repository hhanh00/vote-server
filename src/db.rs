use anyhow::{anyhow, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension as _};
use zcash_vote::{proof::ValidationResult, Election, Hash};

pub const DB_FILE: &str = "vote.db";
pub const REFDATA_FILE: &str = "refdata.db";

pub fn create_db(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS votes(
        id_vote INTEGER PRIMARY KEY NOT NULL,
        id_election INTEGER NOT NULL,
        sig_hash BLOB NOT NULL,
        amount INTEGER NOT NULL,
        candidate INTEGER NOT NULL,
        data BLOB NOT NULL)",
        [],
    )?;
    connection.execute(
        "CREATE TABLE IF NOT EXISTS nfs(
            id_nf INTEGER PRIMARY KEY NOT NULL,
            vote INTEGER NOT NULL,
            hash BLOB NOT NULL)",
        [],
    )?;
    Ok(())
}

pub fn store_ballot(
    election: &Election,
    ballot: &ValidationResult,
    ballot_bytes: &[u8],
    pool: &Pool<SqliteConnectionManager>,
) -> Result<Hash> {
    let mut connection = pool.get().unwrap();
    let db_tx = connection.transaction()?;
    let candidate = u32::from_le_bytes(ballot.candidate.clone().try_into().unwrap());
    db_tx.execute(
        "INSERT INTO votes(id_election, sig_hash, amount, candidate, data)
    VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            election.id,
            &ballot.sig_hash,
            ballot.amount,
            candidate,
            ballot_bytes
        ],
    )?;
    let id_vote = db_tx.last_insert_rowid();
    for nf in ballot.nfs.iter() {
        let dup = db_tx
            .query_row("SELECT 1 FROM nfs WHERE hash = ?1", [nf], |_r| Ok(()))
            .optional()?;
        if dup.is_some() {
            anyhow::bail!("Duplicate vote");
        }
        db_tx.execute(
            "INSERT INTO nfs(vote, hash) VALUES (?1, ?2)",
            params![id_vote, nf],
        )?;
    }
    db_tx.commit()?;
    tracing::debug!("{:?}", ballot);
    tracing::info!("Validated & Stored");

    Ok(ballot.sig_hash)
}

pub fn get_ballot_bytes(hash: &[u8], pool: &Pool<SqliteConnectionManager>) -> Result<Vec<u8>> {
    let connection = pool.get().unwrap();
    let ballot_bytes = connection.query_row("SELECT data FROM votes WHERE sig_hash = ?1",
    [hash], |r| r.get::<_, Vec<u8>>(0)).optional()?;
    let ballot_bytes = ballot_bytes.ok_or(anyhow!("Not found"))?;
    Ok(ballot_bytes)
}
