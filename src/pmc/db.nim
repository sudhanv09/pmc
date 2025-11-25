import std/[strutils, times, os, paths]
import db_connector/db_sqlite
import indexer
import utils

type WatchHistory* = object
  id*: string
  mediaId*: string
  mediaType*: string
  progress*: int
  complete*: bool
  watchedAt*: string

var db: DbConn

proc get_db*(): DbConn =
  return db

proc db_create*() =
  db.exec(
    sql"""
    CREATE TABLE IF NOT EXISTS watch_history (
      id TEXT PRIMARY KEY,
      media_id TEXT NOT NULL,
      media_type TEXT NOT NULL,
      progress INTEGER NOT NULL,
      complete BOOLEAN NOT NULL,
      watched_at TEXT NOT NULL
    );
    """)

  db.exec(sql"""
    CREATE TABLE IF NOT EXISTS movies (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      path TEXT NOT NULL,
      size INTEGER NOT NULL,
      created_at TEXT NOT NULL
    );
    """)

  db.exec(sql"""
    CREATE TABLE IF NOT EXISTS shows (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL
    );
    """)
    
  db.exec(sql"""
    CREATE TABLE IF NOT EXISTS seasons (
      id TEXT PRIMARY KEY,
      show_id TEXT NOT NULL,
      number INTEGER NOT NULL,
      FOREIGN KEY(show_id) REFERENCES shows(id)
    );
    """)
    
  db.exec(sql"""
    CREATE TABLE IF NOT EXISTS episodes (
      id TEXT PRIMARY KEY,
      season_id TEXT NOT NULL,
      name TEXT NOT NULL,
      path TEXT NOT NULL,
      size INTEGER NOT NULL,
      created_at TEXT NOT NULL,
      FOREIGN KEY(season_id) REFERENCES seasons(id)
    );
    """)

proc db_init*(path: string = "pmc.db"): DbConn =
  let db_exists = fileExists(path)
  db = open(path, "", "", "")
  if not db_exists:
    db_create()
  return db

# Watch history CRUD
proc save_state*(wh: WatchHistory) =
  exec(
    db,
    sql"""INSERT INTO watch_history (id, media_id, media_type, progress, complete, watched_at) VALUES (?, ?, ?, ?, ?, ?)""",
    wh.id,
    wh.mediaId,
    wh.mediaType,
    $wh.progress,
    $wh.complete,
    wh.watchedAt,
  )

proc get_media_history*(mediaId: string): seq[WatchHistory] =
  for row in fastRows(
    db,
    sql"""SELECT id, media_id, media_type, progress, complete, watched_at FROM watch_history WHERE media_id = ?""",
    mediaId,
  ):
    result.add WatchHistory(
      id: row[0],
      mediaId: row[1],
      mediaType: row[2],
      progress: parseInt(row[3]),
      complete: parseBool(row[4]),
      watchedAt: row[5],
    )

proc get_recent_watches*(limit: int = 10): seq[WatchHistory] =
  for row in fastRows(
    db,
    sql"""SELECT id, media_id, media_type, progress, complete, watched_at FROM watch_history ORDER BY watched_at DESC LIMIT ?""",
    $limit,
  ):
    result.add WatchHistory(
      id: row[0],
      mediaId: row[1],
      mediaType: row[2],
      progress: parseInt(row[3]),
      complete: parseBool(row[4]),
      watchedAt: row[5],
    )

proc save_media_playback*(media_id: string, media_type: string, progress: int, is_completed: bool) =
  if progress < 0 and not is_completed:
    return

  let watch_id = randomId(7, "abcdefghijklmnopqrstuvwxyz")
  let entry = WatchHistory(
    id: watch_id, media_id: media_id, media_type: media_type, progress: progress, complete: is_completed, watched_at: $now()
  )
  
  save_state(entry)

# Library CRUD
proc get_all_movies*(): seq[Movie] =
  for row in fastRows(db, sql"""SELECT id, name, path, size, created_at FROM movies"""):
    result.add Movie(
      id: row[0],
      name: row[1],
      path: Path(row[2]),
      size: parseInt(row[3]),
      created_at: parse(row[4], "yyyy-MM-dd'T'HH:mm:sszzz"),
    )

proc get_all_shows*(): seq[Show] =
  var shows: seq[Show]
  for row in fastRows(db, sql"""SELECT id, name FROM shows"""):
    var show = Show(id: row[0], name: row[1], seasons: @[])
    for season_row in fastRows(
      db,
      sql"""SELECT id, number FROM seasons WHERE show_id = ?""",
      show.id,
    ):
      var season = Season(
        id: season_row[0], number: parseInt(season_row[1]), episodes: @[]
      )
      for episode_row in fastRows(
        db,
        sql"""SELECT id, name, path, size, created_at FROM episodes WHERE season_id = ?""",
        season.id,
      ):
        season.episodes.add Episode(
          id: episode_row[0],
          name: episode_row[1],
          path: Path(episode_row[2]),
          size: parseInt(episode_row[3]),
          created_at: parse(episode_row[4], "yyyy-MM-dd'T'HH:mm:sszzz"),
        )
      show.seasons.add(season)
    shows.add(show)
  return shows

proc save_movie*(movie: Movie) =
  exec(
    db,
    sql"""INSERT OR REPLACE INTO movies (id, name, path, size, created_at) VALUES (?, ?, ?, ?, ?)""",
    movie.id,
    movie.name,
    $movie.path,
    $movie.size,
    $movie.created_at,
  )

proc save_show*(show: Show) =
  exec(db, sql"""INSERT OR REPLACE INTO shows (id, name) VALUES (?, ?)""", show.id, show.name)

proc save_season*(season: Season, show_id: string) =
  exec(
    db,
    sql"""INSERT OR REPLACE INTO seasons (id, show_id, number) VALUES (?, ?, ?)""",
    season.id,
    show_id,
    $season.number,
  )

proc save_episode*(episode: Episode, season_id: string) =
  exec(
    db,
    sql"""INSERT OR REPLACE INTO episodes (id, season_id, name, path, size, created_at) VALUES (?, ?, ?, ?, ?, ?)""",
    episode.id,
    season_id,
    episode.name,
    $episode.path,
    $episode.size,
    $episode.created_at,
  )
