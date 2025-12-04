use std::{fmt::Display, num::TryFromIntError, str::FromStr, time::Instant};

use chrono::{DateTime, Utc};
use homestat_wire::{Number, Reading, WireMessage, WireMessageDisplay, WithTimestamp};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};

// small macro to avoid repetitive return-type impl-trait
macro_rules! SqliteExecutor {
    ()
        =>
    {
        impl
        sqlx::Executor<'_, Database = sqlx::Sqlite> +
        sqlx::Acquire<'_, Database = sqlx::Sqlite>
    }
}

/// Environment variable that must contain the database url.
const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";

/// Gets database URL from environment variable.
pub fn get_db_url() -> Result<String, std::env::VarError> {
    std::env::var(DATABASE_URL_ENV_VAR)
}

#[derive(Debug, Clone)]
pub struct HomestatDb {
    pool: SqlitePool,
}

impl HomestatDb {
    pub async fn new(url: impl AsRef<str>) -> Result<Self, sqlx::Error> {
        let options =
            SqliteConnectOptions::from_str(url.as_ref())?.journal_mode(SqliteJournalMode::Delete);
        // .foreign_keys(true)
        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }
}

impl AsRef<SqlitePool> for HomestatDb {
    fn as_ref(&self) -> &SqlitePool {
        &self.pool
    }
}

/// A single data point from a Pico.
#[derive(Debug)]
pub struct HomestatRecord {
    /// ID of the Pico that sent this record.
    pub source_id: u8,
    /// Timestamp when the record was received.
    pub recv_timestamp: DateTime<Utc>,
    /// Wire message
    pub wire_message: WireMessage,
}

impl Display for HomestatRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pico {} at {}: {}",
            self.source_id,
            self.recv_timestamp.to_rfc3339(),
            WireMessageDisplay(&self.wire_message)
        )
    }
}

#[derive(Debug)]
struct FetchRecord {
    recv_timestamp: i64,
    source: i64,
    micros: i64,
    celcius_whole: Option<i64>,
    celcius_tenth: Option<i64>,
    humidity_whole: Option<i64>,
    humidity_tenth: Option<i64>,
    error: Option<String>,
}

impl TryFrom<FetchRecord> for HomestatRecord {
    type Error = DbError;

    fn try_from(r: FetchRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            source_id: r.source.try_into()?,
            recv_timestamp: DateTime::from_timestamp_secs(r.recv_timestamp)
                .ok_or(DbError::DateTimeConvert(r.recv_timestamp))?,
            wire_message: WithTimestamp {
                micros: r.micros.try_into()?,
                inner: match r.error.is_none() {
                    true => Ok(Reading {
                        temperature: Number {
                            whole: r.celcius_whole.ok_or(DbError::DbInvalid)?.try_into()?,
                            tenths: r.celcius_tenth.ok_or(DbError::DbInvalid)?.try_into()?,
                        },
                        humidity: Number {
                            whole: r.humidity_whole.ok_or(DbError::DbInvalid)?.try_into()?,
                            tenths: r.humidity_tenth.ok_or(DbError::DbInvalid)?.try_into()?,
                        },
                    }),
                    false => Err(serde_json::from_str(&r.error.ok_or(DbError::DbInvalid)?)?),
                },
            },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Sqlx error: {0:?}")]
    Db(#[from] sqlx::Error),
    #[error("Unable to convert to integer: {0:?}")]
    IntConvert(#[from] TryFromIntError),
    #[error("Unable to convert to datetime: {0}")]
    DateTimeConvert(i64),
    #[error("Unable to encode error as JSON: {0:?}")]
    Serde(#[from] serde_json::Error),
    #[error("DB invalid")]
    DbInvalid,
}

impl HomestatRecord {
    pub async fn insert(&self, executor: SqliteExecutor!()) -> Result<(), DbError> {
        let timestamp = self.recv_timestamp.timestamp();
        let micros: i64 = self
            .wire_message
            .micros
            .try_into()
            .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;

        let mut transaction = executor.begin().await?;

        match &self.wire_message.inner {
            Ok(reading) => {
                // insert
                sqlx::query!(
                    r#"
                    INSERT INTO reading
                        (celcius_whole, celcius_tenth, humidity_whole, humidity_tenth)
                    VALUES (?, ?, ?, ?);"#,
                    reading.temperature.whole,
                    reading.temperature.tenths,
                    reading.humidity.whole,
                    reading.humidity.tenths,
                )
                .execute(&mut *transaction)
                .await?;

                // get row id
                let reading_id = sqlx::query!(
                    r#"
                    SELECT id
                    FROM reading
                    WHERE (
                        celcius_whole = ?
                        AND celcius_tenth = ?
                        AND humidity_whole = ?
                        AND humidity_tenth = ?
                    );"#,
                    reading.temperature.whole,
                    reading.temperature.tenths,
                    reading.humidity.whole,
                    reading.humidity.tenths,
                )
                .fetch_one(&mut *transaction)
                .await?
                .id;

                // insert
                sqlx::query!(
                    r#"
                    INSERT INTO receive
                        (recv_timestamp, source, micros, reading)
                    VALUES (?, ?, ?, ?);"#,
                    timestamp,
                    self.source_id,
                    micros,
                    reading_id
                )
                .execute(&mut *transaction)
                .await?;
            }
            Err(error) => {
                let error = serde_json::to_string(error)?;

                // insert
                sqlx::query!(
                    r#"
                    INSERT INTO error
                        (error)
                    VALUES (?);"#,
                    error
                )
                .execute(&mut *transaction)
                .await?;

                // get row id
                let error_id = sqlx::query!(
                    r#"
                    SELECT id
                    FROM error
                    WHERE (
                        error = ?
                    );"#,
                    error
                )
                .fetch_one(&mut *transaction)
                .await?
                .id;

                // insert
                sqlx::query!(
                    r#"
                    INSERT INTO receive
                        (recv_timestamp, source, micros, error)
                    VALUES (?, ?, ?, ?);"#,
                    timestamp,
                    self.source_id,
                    micros,
                    error_id
                )
                .execute(&mut *transaction)
                .await?;
            }
        };

        transaction.commit().await?;

        Ok(())
    }

    const DEFAULT_FETCH_LIMIT: u64 = 100;

    pub async fn fetch(
        executor: SqliteExecutor!(),
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<Result<Self, DbError>>, DbError> {
        let limit: i64 = limit.unwrap_or(Self::DEFAULT_FETCH_LIMIT).try_into()?;
        let offset: i64 = offset.unwrap_or(0).try_into()?;
        let rows = sqlx::query_as!(
            FetchRecord,
            r#"
            SELECT
                recv_timestamp, source, micros,
                reading.celcius_whole, reading.celcius_tenth,
                reading.humidity_whole, reading.humidity_tenth,
                error.error
            FROM receive
            LEFT JOIN reading ON receive.reading = reading.id
            LEFT JOIN error ON receive.error = error.id
            LIMIT ?
            OFFSET ?;"#,
            limit,
            offset
        )
        .fetch_all(executor)
        .await?;

        let rows = rows.into_iter().map(TryInto::try_into).collect::<Vec<_>>();

        Ok(rows)
    }
}

pub fn fetch_and_print_all(url: &str, limit: Option<u64>) -> Result<(), DbError> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let pool = sqlx::SqlitePool::connect(url).await?;

        let mut offset = 0;
        let mut total = 0;
        let time = Instant::now();
        loop {
            let records = HomestatRecord::fetch(&pool, limit, Some(offset)).await?;

            let num_records = records.len();
            total += num_records;
            offset += num_records as u64;

            for record in records {
                match record {
                    Ok(record) => println!("{}", record),
                    Err(e) => println!("error fetching: {:?}", e),
                }
            }

            if num_records < limit.unwrap_or(HomestatRecord::DEFAULT_FETCH_LIMIT) as usize {
                break;
            }
        }

        let duration = time.elapsed();

        println!("fetched {total} records in {:.5}s", duration.as_secs_f64());

        Ok::<(), DbError>(())
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use homestat_wire::{ChecksumError, Number, ReadError, Reading, WithTimestamp};
    use sqlx::SqlitePool;

    // #[sqlx::test]
    #[tokio::test]
    async fn basic_test(/* pool: SqlitePool */) -> anyhow::Result<()> {
        let pool = SqlitePool::connect(&get_db_url().unwrap()).await?;

        let record = HomestatRecord {
            source_id: 1,
            recv_timestamp: DateTime::<Utc>::from(SystemTime::now()),
            wire_message: WithTimestamp {
                micros: 1234567,
                inner: Ok(Reading {
                    temperature: Number {
                        whole: 12,
                        tenths: 3,
                    },
                    humidity: Number {
                        whole: 45,
                        tenths: 6,
                    },
                }),
            },
        };

        record.insert(&pool).await?;

        let record = HomestatRecord {
            source_id: 1,
            recv_timestamp: DateTime::<Utc>::from_timestamp_millis(123456789).unwrap(),
            wire_message: WithTimestamp {
                micros: 2345678,
                inner: Err(ReadError::Checksum(ChecksumError {
                    expected: 123,
                    actual: 45,
                })),
            },
        };

        record.insert(&pool).await?;

        Ok(())
    }
}
