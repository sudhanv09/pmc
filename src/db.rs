#![allow(warnings)]

use crate::library::{MediaType, WatchEntry};
use chrono::{DateTime, Local};
use colored::Colorize;
use nanoid::nanoid;
use sqlx::sqlite::SqliteRow;
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

        let pool = SqlitePoolOptions::new()
            .connect(&format!("sqlite:{}", db_path.display()))
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS watch_history (
                id TEXT PRIMARY KEY,
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

    fn row_to_watch_entry(&self, row: Vec<SqliteRow>) -> Vec<WatchEntry> {
        row.into_iter()
            .map(|row| {
                let id = row.get("id");
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
                    id,
                    media_id,
                    media_type,
                    progress,
                    complete,
                    watched_at,
                }
            })
            .collect::<Vec<WatchEntry>>()
    }

    pub async fn save_state(&self, entry: &WatchEntry) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO watch_history (id, media_id, media_type, progress, complete, watched_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&entry.id)
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
            SELECT *
            FROM watch_history
            ORDER BY watched_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let entries = self.row_to_watch_entry(rows);
        Ok(entries)
    }

    pub async fn get_media_history(&self, id: String) -> sqlx::Result<Vec<WatchEntry>> {
        let rows = sqlx::query(
            r#"
                select *
                FROM watch_history
                WHERE id = ?
                "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let entries = self.row_to_watch_entry(rows);
        Ok(entries)
    }
}
