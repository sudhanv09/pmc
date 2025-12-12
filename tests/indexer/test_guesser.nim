import pmc/utils
import unittest

suite "guessSeason tests":
  test "empty string":
    check guessSeason("") == 0

  test "S01E02 format":
    check guessSeason("MyShow.S01E02.mkv") == 1
    check guessSeason("another-S10E05.avi") == 10

  test "Season N format":
    check guessSeason("MyShow Season 2 Episode 3.mp4") == 2
    check guessSeason("Title_Season_12_Pilot") == 12

  test "short Sx format":
    check guessSeason("CoolSeries S3 Special.mp4") == 3
    check guessSeason("weird.s7.end.mkv") == 7

  test "mixed text but no season":
    check guessSeason("RandomMovie2022") == 0
    check guessSeason("Documentary.Part.3") == 0

suite "guessEpisode":
  test "SxxExx formats":
    check guessEpisode("MyShow.S01E02.mkv") == 2
    check guessEpisode("another-S10E05.avi") == 5

  test "x notation":
    check guessEpisode("Show.1x03.avi") == 3
    check guessEpisode("Drama.12x07.mkv") == 7

  test "Ep/Episode formats":
    check guessEpisode("Cartoon Ep05.mp4") == 5
    check guessEpisode("Cartoon Episode 6.mp4") == 6
    check guessEpisode("Cartoon.Episode_12.mkv") == 12

  test "no match":
    check guessEpisode("RandomMovie2022") == 0
    check guessEpisode("SeasonOnly.S02.Special") == 0

suite "guessMovieName":
  test "simple name":
    check guessMovieName("MyMovie.mkv") == "MyMovie"
    check guessMovieName("AnotherMovie.mp4") == "AnotherMovie"

  test "with year":
    check guessMovieName("MyMovie (2022).mkv") == "MyMovie"
    check guessMovieName("AnotherMovie (2023).mp4") == "AnotherMovie"
    check guessMovieName("Black Swan (2010) (1080p BluRay x265 afm72).mkv") == "Black Swan"

  test "with year and episode":
    check guessMovieName("Troy.Director's.Cut.2004.Bluray.1080P.AV1.OPUS.5.1-DECK.mkv") == "Troy Director's Cut"
