#[macro_use]
extern crate rocket;

use anyhow::Result;
use clap::Parser;
use clap_repl::reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory};
use clap_repl::{ClapEditor, ReadCommandOutput};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rocket::http::Method;
use rocket::{
    http::Status,
    response::status::Custom,
    State,
};
use std::fs;
use std::io::Read;
use rocket_cors::{AllowedOrigins, CorsOptions};
use vote_server::db::{create_db, get_ballot_bytes, store_ballot, DB_FILE};
use vote_server::{execute, ELECTIONS};
use vote_server::validate::validate;

#[get("/nsm-nu7")]
fn nsm() -> String {
    serde_json::to_string(&ELECTIONS[&1]).unwrap()
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
    id: u32,
    input: &[u8],
    pool: &State<Pool<SqliteConnectionManager>>,
) -> Result<String, Custom<String>> {
    execute!({
    let election = &ELECTIONS[&id];
    let ballot = validate(&election, input)?;
    let hash = store_ballot(&election, &ballot, input, pool)?;
    Ok(hex::encode(&hash))
})
}

#[get("/ballot/<hash>")]
fn get_ballot(hash: String, pool: &State<Pool<SqliteConnectionManager>>) -> Result<String, Custom<String>> {
    execute!({
        let hash = hex::decode(&hash)?;
        let ballot_bytes = get_ballot_bytes(&hash, pool)?;
        Ok(hex::encode(ballot_bytes))
    })
}

#[get("/results")]
fn results(pool: &State<Pool<SqliteConnectionManager>>) -> Result<String, Custom<String>> {
    let results =
        get_results(pool).map_err(|e| Custom(Status::InternalServerError, e.to_string()))?;
    Ok(results)
}

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    cli_main().await?;

    Ok(())
}

#[derive(Parser, Clone, Debug)]
#[command(name = "")]
pub enum Command {
    StartServer,
    Validate { id: u32, filename: String },
}

async fn process_command(command: Command) -> Result<()> {
    match command {
        Command::StartServer => {
            start_server().await?;
        }
        Command::Validate { id, filename } => {
            let election = &ELECTIONS[&id];
            let mut file = fs::File::open(&filename)?;
            let mut data = String::new();
            file.read_to_string(&mut data)?;
            let ballot = validate(election, &hex::decode(data)?)?;
            println!("hash: {}, candidate: {}, amount: {}",
                hex::encode(ballot.sig_hash),
                u32::from_le_bytes(ballot.candidate.try_into().unwrap()), ballot.amount);
        }
    }
    Ok(())
}

pub async fn cli_main() -> Result<()> {
    let prompt = DefaultPrompt {
        left_prompt: DefaultPromptSegment::Basic("vote-cli".to_owned()),
        ..DefaultPrompt::default()
    };
    let mut rl = ClapEditor::<Command>::builder()
        .with_prompt(Box::new(prompt))
        .with_editor_hook(|reed| {
            reed.with_history(Box::new(
                FileBackedHistory::with_file(10000, "/tmp/vote-cli-history".into())
                    .unwrap(),
            ))
        })
        .build();
    loop {
        match rl.read_command() {
            ReadCommandOutput::Command(c) => process_command(c).await?,
            ReadCommandOutput::EmptyLine => (),
            ReadCommandOutput::ClapError(e) => {
                e.print().unwrap();
            }
            ReadCommandOutput::ShlexError => {
                println!(
                    "input was not valid and could not be processed",
                );
            }
            ReadCommandOutput::ReedlineError(e) => {
                panic!("{e}");
            }
            ReadCommandOutput::CtrlC | ReadCommandOutput::CtrlD => break,
        }
    }

    Ok(())
}

pub async fn start_server() -> Result<()> {
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
        .mount("/", routes![nsm, submit, get_ballot, results])
        .attach(cors.to_cors().unwrap())
        .launch()
        .await?;
    Ok(())
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
