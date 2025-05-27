#![allow(warnings)]

use crate::library::{MediaType, WatchEntry};
use chrono::{DateTime, Local};
use colored::Colorize;
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use std::sync::OnceLock;

static DB_CTX: OnceLock<Db> = OnceLock::new();

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn init(db_path: &str) -> sqlx::Result<()> {
        let db_path = if db_path.starts_with("sqlite://") {
            &db_path[9..]
        } else {
            db_path
        };

        let db_path = std::path::Path::new(db_path);
        
        let pool = SqlitePoolOptions::new().connect(&format!("sqlite:{}", db_path.display())).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS watch_history (
                media_id TEXT NOT NULL,
                media_type TEXT NOT NULL,
                progress INTEGER NOT NULL,
                complete BOOLEAN NOT NULL,
                watched_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        DB_CTX
            .set(Db { pool })
            .map_err(|_| sqlx::Error::Configuration("DB already initialized".into()))?;

        Ok(())
    }

    pub fn get() -> &'static Db {
        DB_CTX.get().expect("DB not initialized")
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn save_state(&self, entry: &WatchEntry) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO watch_history (media_id, media_type, progress, complete, watched_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(&entry.media_id)
        .bind(match entry.media_type {
            MediaType::Movie => "Movie",
            MediaType::Show => "Show",
        })
        .bind(entry.progress)
        .bind(entry.complete)
        .bind(entry.watched_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_recent_watches(&self, limit: i64) -> sqlx::Result<Vec<WatchEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT media_id, media_type, progress, complete, watched_at
            FROM watch_history
            ORDER BY watched_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .into_iter()
            .map(|row| {
                let media_id: String = row.get("media_id");
                let media_type_str: String = row.get("media_type");
                let progress: i16 = row.get("progress");
                let complete: bool = row.get("complete");
                let watched_at_str: String = row.get("watched_at");

                let media_type = match media_type_str.as_str() {
                    "Movie" => MediaType::Movie,
                    "Show" => MediaType::Show,
                    _ => panic!("Unknown media type"),
                };

                let watched_at = DateTime::parse_from_rfc3339(&watched_at_str)
                    .unwrap()
                    .with_timezone(&Local);

                WatchEntry {
                    media_id,
                    media_type,
                    progress,
                    complete,
                    watched_at,
                }
            })
            .collect::<Vec<WatchEntry>>();

        Ok(entries)
    }
}
