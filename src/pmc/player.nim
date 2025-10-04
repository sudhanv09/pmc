import std/[asyncdispatch, options, times, os]
import mpv, db
type
  MediaKind = enum
    MovieKind, EpisodeKind

  MediaItem = object
    id: string
    name: string
    path: string
    kind: MediaKind

proc play_media*(item: MediaItem) {.async.} =
  echo "Now playing: ", item.name
  
  let mpv_process = spawn_mpv()
  sleep(1000)
  
  await mpv_init()
  await mpv.play_file(item.path)
  
  updateState(status = Started, filename = item.path, position = 0)

  asyncCheck monitor_playback(item)

proc monitor_playback(item: MediaItem) {.async.} = 
    while true:
        await sleepAsync(2000)
        let state = getState()
        case state.status:
            of Completed, FileEnded:
                echo "Playback complete"
                let media_type = if item.kind == MovieKind: "movie" else: "episode"
                save_media_playback(item.id, media_type, 100, true)
                echo "Playback status saved to database"
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
