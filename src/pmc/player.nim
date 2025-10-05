import std/[asyncdispatch, strformat, times, os]
import mpv, db
type
  MediaKind* = enum
    MovieKind, EpisodeKind

  MediaItem* = object
    id*: string
    name*: string
    path*: string
    kind*: MediaKind

proc monitor_playback(item: MediaItem) {.async.} = 
  echo "Starting monitoring"
  while true:
    await sleepAsync(3000)
    let state = getState()
    case state.status:
      of Completed:
        echo "Playback complete"
        let media_type = if item.kind == MovieKind: "movie" else: "episode"
        save_media_playback(item.id, media_type, 100, true)
        echo "Playback status saved to database"
        break
      of FileEnded:
        echo "File ended"
        # let media_type = if item.kind == MovieKind: "movie" else: "episode"
        # save_media_playback(item.id, media_type, 100, true)
        # echo "Playback status saved to database"
        break
      of Paused:
        echo "Playback paused"
      of RequestedQuit, Exited:
        echo "Playback stopped by user"
        let media_type = if item.kind == MovieKind: "movie" else: "episode"
        save_media_playback(item.id, media_type, int(state.position), false)
        echo fmt"Playback progress ({state.position:.2f}%) saved to database"
        break
      else: 
        # Continue monitoring
        if state.position > 0:
            echo fmt"Playing at {state.position:.2f}%"
        discard

proc play_media*(item: MediaItem) {.async.} =  
  let mpv_process = spawn_mpv()
  await sleepAsync(1000)
  
  await mpv_init()
  await play_file(item.path)
  await sleepAsync(1000)
  asyncCheck mpv_start_monitoring()
  
  updateState(status = Started, filename = item.path, position = 0)

  await monitor_playback(item)