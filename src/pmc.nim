import pmc/[db, indexer, player, utils]
import argparse
import std/[asyncdispatch, os, strformat, strutils, sugar]
import cli/pprint

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

proc play_movie() = discard
proc play_show() = discard

proc list_media() =
  echo "Movies:"
  pprint(get_all_movies().map(m => guessMovieName(m.name)), cols=2)
  
  echo "\nShows:"
  pprint(get_all_shows().map(s => s.name), cols=2)


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
  command("resume"):
    run:
      echo "resuming playback"
  command("sync"):
    run:
      echo "reindexing"

args.run()
user_init()