import std/[asyncdispatch, strformat, algorithm, sequtils, sugar, options]
import ../cli/select
import mpv, db, indexer

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
        let media_type = if item.kind == MovieKind: "movie" else: "episode"
        save_media_playback(item.id, media_type, 100, true)
        echo "Episode completed, moving to next in playlist"
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

proc flatten_show(show: Show): seq[Episode] = 
  var eps: seq[Episode] = @[]
  for season in show.seasons:
    for episode in season.episodes:
      eps.add(episode)
  
  return eps

proc find_last_watched_episode(show: Show, episodes: seq[Episode]): (Episode, int) =
  let recent_watch = get_most_recent_episode_watch(show.id)
  
  if recent_watch.isSome():
    let watch = recent_watch.get()
    # Find the episode in the list
    var episode_index = -1
    for i, ep in episodes:
      if ep.id == watch.mediaId:
        episode_index = i
        break
    
    if episode_index >= 0:
      let episode = episodes[episode_index]
      if watch.progress < 100:
        # Incomplete, resume from this episode
        return (episode, watch.progress)
      else:
        # Complete, start from next episode
        if episode_index + 1 < episodes.len:
          return (episodes[episode_index + 1], 0)
        else:
          # Last episode was completed, start from beginning
          return (episodes[0], 0)
  
  # No history, start from first episode
  if episodes.len > 0:
    return (episodes[0], 0)
  else:
    raise newException(ValueError, "No episodes found for show")

proc play_media(item: MediaItem, startPosition: float = 0.0, playlist: seq[string] = @[], playlistIndex: int = 0) {.async.} =  
  let mpv_process = spawn_mpv()
  await sleepAsync(1000)
  
  await mpv_init()
  
  if playlist.len > 0:
    await play_file(playlist[0])
    if playlist.len > 1:
      await buildPlaylist(playlist[1..^1])
    await sleepAsync(500)
    await set_playlist_pos(playlistIndex)
  else:
    await play_file(item.path)
  
  await sleepAsync(1000)
  asyncCheck mpv_start_monitoring()
  
  if startPosition > 0.0:
    await seek($startPosition)
  
  updateState(status = Started, filename = item.path, position = startPosition)

  await monitor_playback(item)

proc play_movie*() = 
  let movies = get_all_movies().sortedByIt(it.name)
  let choice = select(movies.map(m => m.name), prompt="Select a movie to play:")
  let movie = movies[choice]
  echo "Playing movie: " & movie.name
  waitFor play_media(MediaItem(id: movie.id, name: movie.name, path: movie.path.string, kind: MovieKind))


proc play_show*() = 
  let shows = get_all_shows().sortedByIt(it.name)
  let choice = select(shows.map(s => s.name), prompt="Which show do you want to watch?")
  let show = shows[choice]
  echo "Playing show: " & show.name
  
  let episodes = flatten_show(show)
  if episodes.len == 0:
    echo "No episodes found for this show"
    return
  
  let (start_episode, start_progress) = find_last_watched_episode(show, episodes)
  
  let playlist = episodes.map(ep => ep.path.string)
  
  var playlist_index = 0
  for i, ep in episodes:
    if ep.id == start_episode.id:
      playlist_index = i
      break
  
  echo fmt"Starting from episode: {start_episode.name} at {start_progress}%"
  
  waitFor play_media(
    MediaItem(id: start_episode.id, name: start_episode.name, path: start_episode.path.string, kind: EpisodeKind),
    float(start_progress),
    playlist,
    playlist_index
  )

proc resume_playback*() =
  discard