import pmc/[db, indexer]
import argparse, os

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
        echo "what do you want to watch?"
  command("resume"):
    run:
      echo "resuming playback"
  command("sync"):
    run:
      echo "reindexing"

args.run()
user_init()