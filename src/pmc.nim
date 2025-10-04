import pmc/[db, indexer, player]
import argparse
import std/[asyncdispatch, os, strformat, strutils]

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
  for movie in get_all_movies():
    echo "  - " & movie.name
  
  echo "Shows:"
  for show in get_all_shows():
    echo "  - " & show.name

proc display_media_choice(): tuple[choice: int, is_movie: bool] =
  let movies = get_all_movies()
  let shows = get_all_shows()
  
  echo "Choose what to watch:"
  echo "[0] Movies"
  echo "[1] TV Shows"
  
  let category_choice = parseInt(readLine(stdin))
  
  if category_choice == 0 and movies.len > 0:
    echo "\nMovies:"
    for i, movie in movies:
      echo fmt"[{i + 2}] {movie.name}"
    
    let movie_choice = parseInt(readLine(stdin))
    if movie_choice >= 2 and movie_choice < 2 + movies.len:
      return (movie_choice - 2, true)
    else:
      echo "Invalid movie choice"
      return (-1, false)
  elif category_choice == 1 and shows.len > 0:
    echo "\nTV Shows:"
    for i, show in shows:
      echo fmt"[{i + 2}] {show.name}"
    
    let show_choice = parseInt(readLine(stdin))
    if show_choice >= 2 and show_choice < 2 + shows.len:
      return (show_choice - 2, false)
    else:
      echo "Invalid show choice"
      return (-1, false)
  else:
    echo "Invalid category choice"
    return (-1, false)

proc get_movie_by_index(index: int): Movie =
  let movies = get_all_movies()
  if index >= 0 and index < movies.len:
    return movies[index]
  else:
    raise newException(ValueError, "Invalid movie index")

proc get_show_by_index(index: int): Show =
  let shows = get_all_shows()
  if index >= 0 and index < shows.len:
    return shows[index]
  else:
    raise newException(ValueError, "Invalid show index")

proc get_episode_paths(show: Show): seq[string] =
  var paths: seq[string] = @[]
  for season in show.seasons:
    for episode in season.episodes:
      paths.add(episode.path.string)
  return paths.sorted()  # Sort episodes by path name

proc get_movie_path(movie: Movie): string =
  return movie.path.string

var args = newParser:
  help("A simple media manager utility which can play your media using MPV")
  command("list"):
    run:
      list_media()
  command("recent"):
    run:
      echo "recently watched movies and shows"
      echo get_recent_watches()
  command("play"):
    flag("--tv")
    flag("--movie")
    run:
      if opts.movie:
        echo "playing movie"
      elif opts.tv:
        echo "playing tv"
      else:
        let (choice, is_movie) = display_media_choice()
        if choice >= 0:
          if is_movie:
            let movie = get_movie_by_index(choice)
            echo fmt"Now playing movie: {movie.name}"
            let movie_path = get_movie_path(movie)
            
            # Create media item for movie
            let media_item = MediaItem(
              id: movie.id,
              name: movie.name,
              path: movie_path,
              kind: MovieKind
            )
            
            # Initialize MPV and start playback
            waitFor play_media(media_item)
          else:
            let show = get_show_by_index(choice)
            echo fmt"Now playing show: {show.name}"
            let episode_paths = get_episode_paths(show)
            
            if episode_paths.len > 0:
              let first_episode_path = episode_paths[0]
              
              # Find the matching episode object to get its ID and name
              var first_episode: Episode
              var found = false
              for season in show.seasons:
                for episode in season.episodes:
                  if episode.path.string == first_episode_path:
                    first_episode = episode
                    found = true
                    break
                if found: break
              
              if found:
                let media_item = MediaItem(
                  id: first_episode.id,
                  name: first_episode.name & " - " & show.name,
                  path: first_episode_path,
                  kind: EpisodeKind
                )
                
                # Initialize MPV and start playback
                waitFor play_media(media_item)
              else:
                echo "Could not find matching episode details"
            else:
              echo "No episodes found for this show"
        else:
          echo "Invalid selection"
  command("resume"):
    run:
      echo "resuming playback"
  command("sync"):
    run:
      echo "reindexing"

args.run()
user_init()