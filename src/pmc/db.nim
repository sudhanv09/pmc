import db_sqlite, std/[strutils, times]
import nanoid

type
  WatchHistory* = object
    id*: string
    mediaId*: string
    mediaType*: string
    progress*: int
    complete*: bool
    watchedAt*: string

var db: DbConn

proc db_init*(path: string = "pmc.db") =
  db = open(path, "", "", "")

proc get_db*(): DbConn = 
    return db

proc db_create*() =
  exec(db, """
    CREATE TABLE IF NOT EXISTS watch_history (
      id TEXT PRIMARY KEY,
      media_id TEXT NOT NULL,
      media_type TEXT NOT NULL,
      progress INTEGER NOT NULL,
      complete BOOLEAN NOT NULL,
      watched_at TEXT NOT NULL
    )
  """)

# Watch history CRUD
proc save_state*(wh: WatchHistory) =
  exec(db, "INSERT INTO watch_history (id, media_id, media_type, progress, complete, watched_at) VALUES (?, ?, ?, ?, ?, ?)",
       wh.id, wh.mediaId, wh.mediaType, $wh.progress, $wh.complete, wh.watchedAt)

proc get_media_history*(mediaId: string): seq[WatchHistory] =
  for row in fastRows(db, "SELECT id, media_id, media_type, progress, complete, watched_at FROM watch_history WHERE media_id = ?", mediaId):
    result.add WatchHistory(
      id: row[0], 
      mediaId: row[1], 
      mediaType: row[2],
      progress: parseInt(row[3]),
      complete: parseBool(row[4]),
      watchedAt: row[5]
    )

proc get_recent_watches*(limit: int = 10): seq[WatchHistory] =
  for row in fastRows(db, "SELECT id, media_id, media_type, progress, complete, watched_at FROM watch_history ORDER BY watched_at DESC LIMIT ?", $limit):
    result.add WatchHistory(
      id: row[0],
      mediaId: row[1], 
      mediaType: row[2],
      progress: parseInt(row[3]),
      complete: parseBool(row[4]),
      watchedAt: row[5]
    )

proc save_media_playback(media_id: string, media_type: string, progress: int, is_completed: bool) = 
    if progress < 0.0 and !is_completed:
        return

    let entry = WatchHistory(
        id: nanoid(10),
        media_id,
        media_type,
        progress,
        is_completed,
        watched_at: now()
    )