use revaer_test_support::postgres::start_postgres;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const EXPECTED_TABLES: &[&str] = &[
    "media_profile",
    "media_target",
    "media_job",
    "media_job_phase",
    "media_job_operation",
    "media_capability_snapshot",
];

const EXPECTED_PROCS: &[&str] = &[
    "media_profile_upsert_v1",
    "media_profile_list_v1",
    "media_job_create_v1",
    "media_job_phase_append_v1",
    "media_capability_snapshot_record_v1",
    "media_job_list_v1",
];

pub(crate) struct MediaTestDb {
    _db: revaer_test_support::postgres::TestDatabase,
    pool: PgPool,
    pub(crate) system_user_public_id: Uuid,
}

impl MediaTestDb {
    pub(crate) const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub(crate) async fn setup_media_db(label: &str) -> anyhow::Result<MediaTestDb> {
    let postgres = match start_postgres() {
        Ok(db) => db,
        Err(err) => {
            eprintln!("skipping {label}: {err}");
            return Err(anyhow::anyhow!("postgres unavailable"));
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(postgres.connection_string())
        .await?;

    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(&pool).await?;

    let system_user_public_id =
        sqlx::query_scalar::<_, Uuid>("SELECT user_public_id FROM app_user LIMIT 1")
            .fetch_one(&pool)
            .await?;

    Ok(MediaTestDb {
        _db: postgres,
        pool,
        system_user_public_id,
    })
}

#[tokio::test]
async fn media_tables_exist() -> anyhow::Result<()> {
    let db = match setup_media_db("media_tables_exist").await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("skipping media_tables_exist: {err}");
            return Ok(());
        }
    };

    let rows = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'media_%' ORDER BY table_name",
    )
    .fetch_all(db.pool())
    .await?;

    for table in EXPECTED_TABLES {
        assert!(
            rows.iter().any(|item| item == table),
            "missing table {table}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn media_procedures_exist() -> anyhow::Result<()> {
    let db = match setup_media_db("media_procedures_exist").await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("skipping media_procedures_exist: {err}");
            return Ok(());
        }
    };

    let rows = sqlx::query(
        "SELECT proname FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'public' AND proname LIKE 'media_%'",
    )
    .fetch_all(db.pool())
    .await?;

    let procedure_names: Vec<String> = rows
        .iter()
        .map(|row| row.try_get::<String, _>("proname"))
        .collect::<Result<Vec<_>, _>>()?;

    for proc_name in EXPECTED_PROCS {
        assert!(
            procedure_names.iter().any(|item| item == proc_name),
            "missing procedure {proc_name}"
        );
    }

    Ok(())
}
