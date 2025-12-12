import pmc/[db, indexer, player]
import argparse
import std/[os, sugar, options]
import cli/[pprint]

let dbCtx = db_init()

proc user_init() =
  if get_all_movies().len == 0 and get_all_shows().len == 0:
    echo "Welcome to PMC! It seems like this is your first run."
    echo "Please enter the absolute path to your media directory (e.g., /home/user/media):"
    var media_dir: string
    while true:
      let input_dir = readLine(stdin)
      if dirExists(input_dir):
        media_dir = input_dir
        save_setting("media_dir", media_dir)
        echo "Media directory set to: " & media_dir
        echo "Indexing your media... This might take a while."
        let library = create_index(media_dir)
        for movie in library.Movies:
          save_movie(movie)
        for show in library.Shows:
          save_show(show)
          for season in show.seasons:
            save_season(season, show.id)
            for episode in season.episodes:
              save_episode(episode, season.id)
        echo "Indexing complete!"
        break
      else:
        echo "Invalid directory. Please enter a valid absolute path:"

proc list_media() =
  echo "Movies:"
  pprint(get_all_movies().map(m => m.name).sorted(), cols=2)
  
  echo "\nShows:"
  pprint(get_all_shows().map(s => s.name), cols=2)

proc recent_watch() =
  let recentHistory = get_recent_watches()
  if recentHistory.len == 0:
    echo "No recent watches found."
    return

  echo "Recently watched:"
  for entry in recentHistory:
    let progressLabel = if entry.complete: "Completed" else: $entry.progress & "%"

    case entry.mediaType:
    of "movie":
      let movie = get_movie_by_id(entry.mediaId)
      let title = if movie.name.len > 0: movie.name else: "Unknown movie"
      echo "- " & title & " (" & progressLabel & ")"
    of "episode":
      let ctx = get_episode_context(entry.mediaId)
      var details = if ctx.showName.len > 0: ctx.showName else: "Unknown show"
      if ctx.seasonNumber > 0:
        details &= " Season " & $ctx.seasonNumber
      if ctx.episodeName.len > 0:
        details &= " - " & ctx.episodeName

      var position = ""
      if ctx.episodeIndex > 0 and ctx.totalEpisodes > 0:
        position = "Episode " & $ctx.episodeIndex & " of " & $ctx.totalEpisodes

      if position.len > 0:
        details &= " (" & position & ")"

      echo "- " & details & " (" & progressLabel & ")"
    else:
      echo "- Unknown media (" & progressLabel & ")"


var args = newParser:
  help("A simple media manager utility which can play your media using MPV")
  command("ls"):
    run:
      list_media()
  command("recent"):
    run:
      recent_watch()
  command("play"):
    flag("--tv")
    flag("--movie")
    run:
      if opts.movie:
        play_movie()
      elif opts.tv:
        play_show()
  command("resume"):
    run:
      resume_playback()
  command("sync"):
    run:
      let media_dir = get_setting("media_dir")
      if media_dir.isNone:
        echo "No media directory configured. Please run pmc without arguments to set it up."
        return
      echo "Syncing library from: " & media_dir.get
      let (addedMovies, addedEpisodes) = sync_library(media_dir.get)
      echo "Sync complete! Added " & $addedMovies & " movies and " & $addedEpisodes & " episodes."

args.run()
user_init()