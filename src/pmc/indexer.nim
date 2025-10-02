import std/[times, paths, nre, parseutils, dirs, sequtils, strutils, os]
import nanoid

type
  Library* = object
    Movies*: seq[Movie]
    Shows*: seq[Show]

  Movie* = object
    id*: string
    name*: string
    path*: Path
    created_at*: DateTime
    size: int64

  Show* = object
    id*: string
    name*: string
    seasons*: seq[Season]

  Season* = object
    id*: string
    number*: int
    episodes*: seq[Episode]

  Episode* = object
    id*: string
    name*: string
    path*: Path
    created_at*: DateTime
    size*: int64

proc guessSeason*(item: string): int =
  let patterns = [
    re"(?i)S(\d{1,2})E\d{1,2}", # S01E02
    re"(?i)Season[ _]?(\d{1,2})", # Season 2
    re"(?i)S(\d{1,2})", # S1
  ]

  for pat in patterns:
    if item.match(pat).isSome:
      let m = item.match(pat).get
      try:
        var season: int
        discard parseInt(m.captures[0], season)
        return season
      except ValueError:
        discard

  return 0

proc isMediaFile(path: string): bool =
  let validExts = @[".mp4", ".mkv", ".mov", ".avi"]
  result = validExts.anyIt(path.toLowerAscii.endsWith(it))

proc index_movies(dir: Path): seq[Movie] =
  for kind, path in walkDir(dir):
    if kind == pcFile and isMediaFile($path):
      let movie = Movie(
        id: generate(size=10),
        name: path.splitPath.tail.string,
        path: path,
        created_at: now(),
        size: getFileSize($path),
      )
      result.add(movie)
    elif kind == pcDir:
        let dirName = path.splitPath.tail.string.toLower()
        if dirName != "featurettes":
            result.add(index_movies(path))

proc index_shows(dir: Path): seq[Show] =
  for kind, showDir in walkDir(dir):
    if kind == pcDir:
      var show = Show(id: generate(size=10), name: showDir.splitPath.tail.string, seasons: @[])

      # collect immediate files (implicit Season 1)
      var season1 = Season(id: generate(size=10), number: 1, episodes: @[])
      for _, epPath in walkDir(showDir, relative = false):
        if fileExists($epPath) and isMediaFile($epPath):
          season1.episodes.add Episode(
            id: generate(size=10),
            name: epPath.splitPath.tail.string,
            path: epPath,
            created_at: now(),
            size: getFileSize($epPath),
          )

      if season1.episodes.len > 0:
        show.seasons.add(season1)

      # collect season subfolders
      for _, seasonDir in walkDir(showDir):
        if dirExists(seasonDir) and seasonDir != showDir:
          let seasonNum = guessSeason($seasonDir)

          var season = Season(id: generate(size=10), number: seasonNum, episodes: @[])
          for _, epPath in walkDir(seasonDir):
            if fileExists($epPath) and isMediaFile($epPath):
              season.episodes.add Episode(
                id: generate(size=10),
                name: epPath.splitPath.tail.string,
                path: epPath,
                created_at: now(),
                size: getFileSize($epPath),
              )

          if season.episodes.len > 0:
            show.seasons.add(season)

      if show.seasons.len > 0:
        result.add(show)

proc create_index*(dir: string): Library =
  result.Movies = index_movies(Path(dir / "Movies"))
  result.Shows = index_shows(Path(dir / "TV"))
