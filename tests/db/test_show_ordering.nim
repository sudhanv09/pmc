import std/[unittest, sequtils]
import pmc/[db, player, utils, indexer]

proc seasonNum(ep: Episode): int =
  result = guessSeason(ep.name)
  if result == 0:
    result = guessSeason(ep.path.string)

proc episodeNum(ep: Episode): int =
  result = guessEpisode(ep.name)
  if result == 0:
    result = guessEpisode(ep.path.string)

suite "Show ordering with flatten_show":
  setup:
    let _ = db_init()

  test "first flattened episode is the smallest season/episode":
    let shows = get_all_shows()
    check shows.len > 0

    for show in shows:
      let eps = flatten_show(show)
      if eps.len == 0:
        continue

      var valid: seq[(int, int, int)] = @[] # (idx, season, episode)
      for i, ep in eps:
        let s = seasonNum(ep)
        let e = episodeNum(ep)
        if s > 0 and e > 0:
          valid.add((i, s, e))

      if valid.len == 0:
        continue

      # Determine minimal season and minimal episode within that season
      var minSeason = valid[0][1]
      for v in valid:
        if v[1] < minSeason:
          minSeason = v[1]

      var minEpisode = high(int)
      for v in valid:
        if v[1] == minSeason and v[2] < minEpisode:
          minEpisode = v[2]

      let firstSeason = seasonNum(eps[0])
      let firstEpisode = episodeNum(eps[0])
      check firstSeason == minSeason
      check firstEpisode == minEpisode

  test "flattened episodes are ordered by season then episode":
    let shows = get_all_shows()
    check shows.len > 0

    for show in shows:
      let eps = flatten_show(show)
      if eps.len == 0:
        continue

      var prevSeason = 0
      var prevEpisode = 0
      var initialized = false

      for ep in eps:
        let s = seasonNum(ep)
        let e = episodeNum(ep)
        if s == 0 or e == 0:
          continue

        if not initialized:
          prevSeason = s
          prevEpisode = e
          initialized = true
          continue

        if s == prevSeason:
          check e >= prevEpisode
        else:
          check s > prevSeason
        prevSeason = s
        prevEpisode = e
