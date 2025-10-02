import argparse

var args = newParser:
    help("A simple media manager utility which can play your media using MPV")
    command("list"):
        run:
            echo "here is the list"
    command("recent"):
        run:
            echo "recently watched movies and shows"
    command("play"):
        flag("--tv")
        flag("--movie")
        run:
            echo "playing movie"
    command("resume"):
        run:
            echo "resuming playback"
    command("sync"):
        run:
            echo "reindexing"
    
args.run()

