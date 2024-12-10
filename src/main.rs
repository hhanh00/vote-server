#[macro_use]
extern crate rocket;

use anyhow::Result;
use clap::Parser;
use clap_repl::reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory};
use clap_repl::ClapEditor;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rocket::http::Method;
use rocket::tokio;
use rocket::{
    http::Status,
    response::status::Custom,
    State,
};
use rocket_cors::{AllowedOrigins, CorsOptions};
use vote_server::db::{create_db, get_ballot_bytes, store_ballot, DB_FILE};
use vote_server::execute;
use vote_server::validate::validate;
use zcash_vote::Election;

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
) -> Result<String, Custom<String>> {
    execute!({
    let election_str = match id {
        1 => ELECTION_STR1,
        // 2 => ELECTION_STR2,
        // 3 => ELECTION_STR3,
        _ => unreachable!(),
    };
    let election = serde_json::from_str::<Election>(election_str).unwrap();
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

    cli_main()?;

    Ok(())
}

#[derive(Parser, Clone, Debug)]
#[command(name = "")]
pub enum Command {
    StartServer,
}

#[tokio::main]
async fn process_command(command: Command) -> Result<()> {
    match command {
        Command::StartServer => {
            start_server().await?;
        }
    }
    Ok(())
}

pub fn cli_main() -> Result<()> {
    let prompt = DefaultPrompt {
        left_prompt: DefaultPromptSegment::Basic("vote-cli".to_owned()),
        ..DefaultPrompt::default()
    };
    let rl = ClapEditor::<Command>::builder()
        .with_prompt(Box::new(prompt))
        .with_editor_hook(|reed| {
            reed.with_history(Box::new(
                FileBackedHistory::with_file(10000, "/tmp/vote-cli-history".into())
                    .unwrap(),
            ))
        })
        .build();
    rl.repl(|command| {
        if let Err(e) = process_command(command) {
            tracing::error!("{e}");
        }
    });
    
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
