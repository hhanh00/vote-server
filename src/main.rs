#[macro_use]
extern crate rocket;

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rocket::http::Method;
use rocket::{
    http::Status,
    response::status::{BadRequest, Custom},
    State,
};
use rocket_cors::{AllowedOrigins, CorsOptions};
use rusqlite::{params, Connection, OptionalExtension};
use zcash_vote::{validate_proof, Election, Hash};

pub const DB_FILE: &str = "vote.db";

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

pub const ELECTION_STR1: &str = include_str!("election.json");

#[get("/nsm-nu7")]
fn nsm() -> &'static str {
    ELECTION_STR1
}

// #[get("/devfund-props")]
// fn devfund2() -> &'static str {
//     ELECTION_STR2
// }

// #[get("/devfund-runoff")]
// fn devfund3() -> &'static str {
//     ELECTION_STR3
// }

#[put("/submit/<id>", data = "<input>")]
fn submit(
    id: u8,
    input: &[u8],
    pool: &State<Pool<SqliteConnectionManager>>,
) -> Result<String, BadRequest<String>> {
    let election_str = match id {
        1 => ELECTION_STR1,
        // 2 => ELECTION_STR2,
        // 3 => ELECTION_STR3,
        _ => unreachable!(),
    };
    let election = serde_json::from_str::<Election>(election_str).unwrap();
    let hash = validate(&election, input, pool).map_err(|e| BadRequest(e.to_string()))?;
    Ok(hex::encode(&hash))
}

#[get("/results")]
fn results(pool: &State<Pool<SqliteConnectionManager>>) -> Result<String, Custom<String>> {
    let results =
        get_results(pool).map_err(|e| Custom(Status::InternalServerError, e.to_string()))?;
    Ok(results)
}

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec![Method::Get, Method::Post, Method::Patch]
                .into_iter()
                .map(From::from)
                .collect(),
        )
        .allow_credentials(true);

    let manager = r2d2_sqlite::SqliteConnectionManager::file(DB_FILE);
    let pool = r2d2::Pool::new(manager).unwrap();
    let connection = pool.get().unwrap();
    create_db(&connection).unwrap();
    rocket::build()
        .manage(pool)
        .mount("/", routes![nsm, submit, results])
        .attach(cors.to_cors().unwrap())
        .launch()
        .await?;
    Ok(())
}

fn validate(
    election: &Election,
    input: &[u8],
    pool: &Pool<SqliteConnectionManager>,
) -> Result<Hash> {
    let mut connection = pool.get().unwrap();
    let db_tx = connection.transaction()?;
    let domain = orchard::pob::domain(election.name.as_bytes());
    let result = validate_proof(&input, domain, &election)?;
    let candidate = u32::from_le_bytes(result.candidate.clone().try_into().unwrap());
    db_tx.execute(
        "INSERT INTO votes(id_election, sig_hash, amount, candidate, data) 
    VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            election.id,
            &result.sig_hash,
            result.amount,
            candidate,
            input
        ],
    )?;
    let id_vote = db_tx.last_insert_rowid();
    for nf in result.nfs.iter() {
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
    println!("{:?}", result);

    println!("Validated");

    Ok(result.sig_hash)
}

fn get_results(pool: &Pool<SqliteConnectionManager>) -> Result<String> {
    let connection = pool.get().unwrap();
    let mut s = connection.prepare("SELECT candidate, amount FROM votes")?;
    let rows = s.query_map([], |r| {
        let candidate = r.get::<_, u32>(0)?;
        let amount = r.get::<_, u64>(1)?;
        Ok((candidate, amount))
    })?;
    let results = rows.collect::<Result<Vec<_>, _>>()?;
    let v = serde_json::to_string(&results)?;
    Ok(v)
}
