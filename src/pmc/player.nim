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

proc monitor_playback(item: MediaItem, episodes: seq[Episode] = @[], current_index: int = -1) {.async.} = 
  var current_item = item
  var current_ep_index = current_index
  echo "Starting monitoring"
  while true:
    await sleepAsync(1500)
    let state = getState()
    case state.status:
      of Completed:
        echo "Playback complete"
        let media_type = if current_item.kind == MovieKind: "movie" else: "episode"
        save_media_playback(current_item.id, media_type, 100, true)
        echo "Playback status saved to database"
        # For episodes, check if there's a next one
        if current_item.kind == EpisodeKind and episodes.len > 0 and current_ep_index >= 0:
          if current_ep_index + 1 < episodes.len:
            let next_ep = episodes[current_ep_index + 1]
            current_item = MediaItem(id: next_ep.id, name: next_ep.name, path: next_ep.path.string, kind: EpisodeKind)
            current_ep_index = current_ep_index + 1
            echo fmt"Transitioning to next episode: {next_ep.name}"
            
            await set_playlist_pos(current_ep_index)
            
            updateState(status = Started, filename = current_item.path, position = 0.0)
          else:
            echo "All episodes completed"
            break
        else:
          break
      of FileEnded:
        # FileEnded can occur during transitions, only treat as complete if position > 98
        if state.position > 98.0:
          echo "Playback complete"
          let media_type = if current_item.kind == MovieKind: "movie" else: "episode"
          save_media_playback(current_item.id, media_type, 100, true)
          echo "Playback status saved to database"
          # For episodes, check if there's a next one
          if current_item.kind == EpisodeKind and episodes.len > 0 and current_ep_index >= 0:
            if current_ep_index + 1 < episodes.len:
              let next_ep = episodes[current_ep_index + 1]
              current_item = MediaItem(id: next_ep.id, name: next_ep.name, path: next_ep.path.string, kind: EpisodeKind)
              current_ep_index = current_ep_index + 1
              echo fmt"Transitioning to next episode: {next_ep.name}"
              
              await set_playlist_pos(current_ep_index)
              
              updateState(status = Started, filename = current_item.path, position = 0.0)
            else:
              echo "All episodes completed"
              break
          else:
            break
      of Paused:
        echo "Playback paused"
      of RequestedQuit, Exited:
        echo "Playback stopped by user"
        let media_type = if current_item.kind == MovieKind: "movie" else: "episode"
        save_media_playback(current_item.id, media_type, int(state.position), false)
        echo fmt"Playback progress ({state.position:.2f}%) saved to database"
        break
      else: 
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

proc play_media(item: MediaItem, startPosition: float = 0.0, playlist: seq[string] = @[], playlistIndex: int = 0, episodes: seq[Episode] = @[]) {.async.} =  
  let mpv_process = spawn_mpv()
  await sleepAsync(2000)
  
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

  await monitor_playback(item, episodes, playlistIndex)

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
    playlist_index,
    episodes
  )

proc resume_playback*() =
  discard