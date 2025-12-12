import std/[asyncdispatch, strformat, algorithm, sequtils, sugar, options]
import ../cli/select
import mpv, db, indexer, utils

type
  MediaKind* = enum
    MovieKind, EpisodeKind

  MediaItem* = ref object
    id*: string
    name*: string
    path*: string
    kind*: MediaKind

proc save_playback_status(item: MediaItem, progress: int, completed: bool) =
  let media_type = if item.kind == MovieKind: "movie" else: "episode"
  save_media_playback(item.id, media_type, progress, completed)
  if completed:
    echo "Playback status saved to database"
  else:
    echo fmt"Playback progress ({progress}%) saved to database"

proc try_transition_to_next_episode(current_item: MediaItem, current_ep_index: int, episodes: seq[Episode]): Future[(bool, MediaItem, int)] {.async.} =
  if current_item.kind == EpisodeKind and episodes.len > 0 and current_ep_index >= 0:
    if current_ep_index + 1 < episodes.len:
      let next_ep = episodes[current_ep_index + 1]
      let new_item = MediaItem(id: next_ep.id, name: next_ep.name, path: next_ep.path.string, kind: EpisodeKind)
      let new_index = current_ep_index + 1
      echo fmt"Transitioning to next episode: {next_ep.name}"
      await set_playlist_pos(new_index)
      updateState(status = Started, filename = new_item.path, position = 0.0)
      return (true, new_item, new_index)
    else:
      echo "All episodes completed"
  return (false, current_item, current_ep_index)

proc try_transition_to_prev_episode(current_item: MediaItem, current_ep_index: int, episodes: seq[Episode]): Future[(bool, MediaItem, int)] {.async.} =
  if current_item.kind == EpisodeKind and episodes.len > 0 and current_ep_index >= 0:
    if current_ep_index - 1 >= 0:
      let prev_ep = episodes[current_ep_index - 1]
      let new_item = MediaItem(id: prev_ep.id, name: prev_ep.name, path: prev_ep.path.string, kind: EpisodeKind)
      let new_index = current_ep_index - 1
      echo fmt"Transitioning to previous episode: {prev_ep.name}"
      await set_playlist_pos(new_index)
      updateState(status = Started, filename = new_item.path, position = 0.0)
      return (true, new_item, new_index)
    else:
      echo "All episodes completed"
  return (false, current_item, current_ep_index)

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
        save_playback_status(current_item, 100, true)
        let (success, new_item, new_index) = await try_transition_to_next_episode(current_item, current_ep_index, episodes)
        if success:
          current_item = new_item
          current_ep_index = new_index
        else:
          break
      of FileEnded:
        if state.position > 98.0:
          echo "Playback complete"
          save_playback_status(current_item, 100, true)
          let (success, new_item, new_index) = await try_transition_to_next_episode(current_item, current_ep_index, episodes)
          if success:
            current_item = new_item
            current_ep_index = new_index
          else:
            break
      of Paused:
        stdout.write("\r\x1b[K")
        stdout.write("Paused")
        stdout.flushFile()
      of RequestedQuit, Exited:
        echo "Playback stopped by user"
        save_playback_status(current_item, int(state.position), false)
        await user_quit()
        break
      of RequestedNext:
        echo "Requested next episode"
        let (success, new_item, new_index) = await try_transition_to_next_episode(current_item, current_ep_index, episodes)
        if success:
          current_item = new_item
          current_ep_index = new_index
        else:
          break
      of RequestedPrev:
        echo "Requested previous episode"
        let (success, new_item, new_index) = await try_transition_to_prev_episode(current_item, current_ep_index, episodes)
        if success:
          current_item = new_item
          current_ep_index = new_index
        else:
          break
      else: 
        if state.position > 0:
            stdout.write("\r\x1b[K")   # move to line start + clear line
            stdout.write(fmt"Playing at {state.position:.2f}%")
            stdout.flushFile()
        discard

proc flatten_show*(show: Show): seq[Episode] = 
  var eps: seq[Episode] = @[]
  
  # Sort seasons by season number
  var sorted_seasons = show.seasons
  sorted_seasons.sort(proc (a, b: Season): int =
    cmp(a.number, b.number)
  )
  
  for season in sorted_seasons:
    var sorted_episodes = season.episodes
    sorted_episodes.sort(proc (a, b: Episode): int =
      cmp(guessEpisode(a.path.string), guessEpisode(b.path.string))
    )
    for episode in sorted_episodes:
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