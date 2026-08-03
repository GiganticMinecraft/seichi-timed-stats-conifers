use diesel_async::{AsyncConnection, AsyncMigrationHarness, AsyncMysqlConnection};
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../database/migrations");

fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("environment variable {name} must be set"))
}

// AsyncMigrationHarness は内部で tokio::task::block_in_place を使うため
// current-thread runtime では動かない。multi_thread を明示しておく
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let url = format!(
        "mysql://{}:{}@{}/{}",
        required_env("DB_USER"),
        required_env("DB_PASSWORD"),
        required_env("DB_HOST_AND_PORT"),
        required_env("DB_DATABASE_NAME"),
    );

    let connection = match AsyncMysqlConnection::establish(&url).await {
        Ok(connection) => connection,
        Err(e) => {
            eprintln!("Failed to establish database connection: {e}");
            std::process::exit(1);
        }
    };

    let mut harness = AsyncMigrationHarness::new(connection);
    // HarnessWithOutput は適用した migration ごとに
    // "Running migration <name>" を出力する (旧 diesel CLI と同じ形式。
    // CI の二重実行テストがこの出力を検証している)
    if let Err(e) =
        HarnessWithOutput::write_to_stdout(&mut harness).run_pending_migrations(MIGRATIONS)
    {
        eprintln!("Failed to run migrations: {e}");
        std::process::exit(1);
    }
}
