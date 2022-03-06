use crate::lib;
use clap::ArgMatches;
use rand::thread_rng;
use sqlx::{migrate, migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite, SqlitePool};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct AppConfig {
    pub blacklist: Option<Vec<String>>,
    pub output: PathBuf,
    pub password_hash: Option<String>,
    pub port: Option<String>,
}

pub struct ServerConfig {
    pub unix: Option<PathBuf>,
    database_uri: String,
}

impl From<&ArgMatches> for AppConfig {
    fn from(matches: &ArgMatches) -> Self {
        let port: Option<String>;

        #[cfg(unix)]
        {
            port = if !matches.is_present("disable-port") {
                Some(matches.value_of("port").unwrap().to_owned())
            } else {
                None
            };
        }

        #[cfg(not(unix))]
        {
            port = Some(matches.value_of("port").unwrap().to_owned());
        }

        Self {
            blacklist: matches
                .values_of("blacklist")
                .map(|values| values.map(|str| str.to_lowercase()).collect()),
            output: PathBuf::from(matches.value_of("output").unwrap()),
            password_hash: matches
                .value_of("password")
                .map(|str| lib::hash_password(&mut thread_rng(), str)),
            port,
        }
    }
}

impl From<&ArgMatches> for ServerConfig {
    fn from(matches: &ArgMatches) -> Self {
        Self {
            unix: matches.value_of("unix").map(PathBuf::from),
            database_uri: format!(
                "sqlite://{}",
                Path::new(matches.value_of("database").unwrap())
                    .join("dekinai.sqlite")
                    .display()
            ),
        }
    }
}

impl ServerConfig {
    pub async fn open_database_pool(&self) -> Result<SqlitePool, sqlx::Error> {
        if !Sqlite::database_exists(&self.database_uri).await? {
            Sqlite::create_database(&self.database_uri).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(num_cpus::get() as u32)
            .min_connections(1)
            .connect(&self.database_uri)
            .await?;

        migrate!().run(&pool).await?;
        Ok(pool)
    }
}
