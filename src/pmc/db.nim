import std/[strutils, times, os, paths, options]
import db_connector/db_sqlite
import indexer
import utils

type
  WatchHistory* = object
    id*: string
    mediaId*: string
    mediaType*: string
    progress*: int
    complete*: bool
    watchedAt*: string

  EpisodeContext* = object
    showName*: string
    seasonNumber*: int
    episodeName*: string
    episodeIndex*: int
    totalEpisodes*: int

var db: DbConn

proc get_db*(): DbConn =
  return db

proc db_create*() =
  db.exec(sql"""
    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );
    """)

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

# Settings CRUD
proc save_setting*(key: string, value: string) =
  db.exec(sql"""INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)""", key, value)

proc get_setting*(key: string): Option[string] =
  for row in fastRows(db, sql"""SELECT value FROM settings WHERE key = ? LIMIT 1""", key):
    return some(row[0])
  return none(string)

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

proc get_most_recent_episode_watch*(showId: string): Option[WatchHistory] =
  # Query watch_history for the most recent episode watch of this show
  # Join through seasons and episodes to find all episode IDs for this show
  for row in fastRows(
    db,
    sql"""
      SELECT wh.id, wh.media_id, wh.media_type, wh.progress, wh.complete, wh.watched_at
      FROM watch_history wh
      INNER JOIN episodes e ON wh.media_id = e.id
      INNER JOIN seasons s ON e.season_id = s.id
      WHERE s.show_id = ? AND wh.media_type = 'episode'
      ORDER BY wh.watched_at DESC
      LIMIT 1
    """,
    showId,
  ):
    return some(WatchHistory(
      id: row[0],
      mediaId: row[1],
      mediaType: row[2],
      progress: parseInt(row[3]),
      complete: parseBool(row[4]),
      watchedAt: row[5],
    ))
  return none(WatchHistory)

proc get_movie_by_id*(movieId: string): Movie =
  for row in fastRows(
    db, sql"""SELECT id, name, path, size, created_at FROM movies WHERE id = ? LIMIT 1""", movieId
  ):
    return Movie(
      id: row[0],
      name: row[1],
      path: Path(row[2]),
      size: parseInt(row[3]),
      created_at: parse(row[4], "yyyy-MM-dd'T'HH:mm:sszzz"),
    )

proc movie_exists_by_path*(path: Path): bool =
  for _ in fastRows(db, sql"""SELECT 1 FROM movies WHERE path = ? LIMIT 1""", $path):
    return true
  return false

proc episode_exists_by_path*(path: Path): bool =
  for _ in fastRows(db, sql"""SELECT 1 FROM episodes WHERE path = ? LIMIT 1""", $path):
    return true
  return false

proc get_show_id_by_name*(name: string): Option[string] =
  for row in fastRows(db, sql"""SELECT id FROM shows WHERE name = ? LIMIT 1""", name):
    return some(row[0])
  return none(string)

proc get_season_id_by_show_and_number*(show_id: string, number: int): Option[string] =
  for row in fastRows(
    db,
    sql"""SELECT id FROM seasons WHERE show_id = ? AND number = ? LIMIT 1""",
    show_id,
    $number,
  ):
    return some(row[0])
  return none(string)

proc get_episode_context*(episodeId: string): EpisodeContext =
  var seasonId: string
  for row in fastRows(
    db, sql"""SELECT name, season_id FROM episodes WHERE id = ? LIMIT 1""", episodeId
  ):
    result.episodeName = row[0]
    seasonId = row[1]

  if seasonId.len == 0:
    return

  var showId: string
  for row in fastRows(
    db, sql"""SELECT number, show_id FROM seasons WHERE id = ? LIMIT 1""", seasonId
  ):
    result.seasonNumber = parseInt(row[0])
    showId = row[1]

  if showId.len > 0:
    for row in fastRows(
      db, sql"""SELECT name FROM shows WHERE id = ? LIMIT 1""", showId
    ):
      result.showName = row[0]

  var counter = 0
  for row in fastRows(
    db,
    sql"""SELECT id FROM episodes WHERE season_id = ? ORDER BY created_at ASC, name ASC""",
    seasonId,
  ):
    inc counter
    if row[0] == episodeId:
      result.episodeIndex = counter

  result.totalEpisodes = counter

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

proc sync_library*(media_dir: string): tuple[movies: int, episodes: int] =
  ## Syncs the library by adding only new files. Returns count of added items.
  var addedMovies = 0
  var addedEpisodes = 0

  let library = create_index(media_dir)

  # Sync movies
  for movie in library.Movies:
    if not movie_exists_by_path(movie.path):
      save_movie(movie)
      inc addedMovies

  # Sync shows
  for show in library.Shows:
    let existingShowId = get_show_id_by_name(show.name)
    let showId = if existingShowId.isSome:
      existingShowId.get
    else:
      save_show(show)
      show.id

    for season in show.seasons:
      let existingSeasonId = get_season_id_by_show_and_number(showId, season.number)
      let seasonId = if existingSeasonId.isSome:
        existingSeasonId.get
      else:
        save_season(season, showId)
        season.id

      for episode in season.episodes:
        if not episode_exists_by_path(episode.path):
          save_episode(episode, seasonId)
          inc addedEpisodes

  return (movies: addedMovies, episodes: addedEpisodes)
