import std/[nre, strutils, random, os]

const
  defaultAlphabet = "_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
  releaseStopWords = [
    "1080p", "720p", "2160p", "480p", "4k",
    "bluray", "bdrip", "dvdrip", "hdrip", "webrip", "webdl", "web",
    "remux", "hdr", "uhd",
    "x264", "x265", "h264", "h265", "hevc",
    "av1", "aac", "ac3", "dts", "opus", "truehd", "atmos", "ddp",
    "cam", "ts", "tc", "r5"
  ]

var rngSeeded = false

proc ensureSeeded() =
  if not rngSeeded:
    randomize()
    rngSeeded = true

proc randomId*(size: int = 10, alphabet: string = defaultAlphabet): string =
  if size <= 0 or alphabet.len == 0:
    return ""
  ensureSeeded()
  result = newString(size)
  for i in 0 ..< size:
    result[i] = alphabet[rand(alphabet.high)]

proc guessSeason*(item: string): int =
  let patterns = [
    re"(?i)S(\d{1,2})E\d{1,2}", # S01E02
    re"(?i)Season[ _]?(\d{1,2})", # Season 2
    re"(?i)S(\d{1,2})", # S1
  ]

  for pat in patterns:
    let mOpt = item.find(pat)
    if mOpt.isSome:
      let m = mOpt.get
      try:
        return parseInt(m.captures[0])
      except ValueError:
        discard

  return 0

proc guessEpisode*(item: string): int =
  let patterns = [
    # 1. S01E02 format
    re"(?i)\bS\d{1,2}E(\d{1,2})\b",
    
    # 2. 1x02 format
    re"(?i)\d{1,2}x(\d{1,2})", 
    
    # 3. Ep02 format
    re"(?i)Ep(?:isode)?[ ._-]?(\d{1,2})" 
  ]

  for pat in patterns:
    let mOpt = item.find(pat)
    if mOpt.isSome:     
      try:
        return parseInt(mOpt.get.captures[0])
      except ValueError:
        discard

  return 0

proc guessMovieName*(item: string): string =
  if item.len == 0:
    return ""

  let parts = splitFile(item)
  var base = parts.name

  if base.len == 0:
    return ""

  # Remove bracketed/parenthetical metadata like "(2022)" or "[1080p]"
  base = base.replace(re"\([^)]*\)", " ")
  base = base.replace(re"\[[^\]]*\]", " ")

  # Normalize common separators to spaces
  for sep in [".", "_", "-", "+"]:
    base = base.replace(sep, " ")

  base = base.replace(re"\s+", " ").strip()
  if base.len == 0:
    return ""

  proc looksLikeYear(token: string): bool =
    if token.len != 4:
      return false
    for ch in token:
      if ch < '0' or ch > '9':
        return false
    let year = parseInt(token)
    return year in 1900 .. 2099

  proc looksLikeQuality(token: string): bool =
    let lower = token.toLowerAscii()
    if lower in releaseStopWords:
      return true
    if lower.len > 1 and lower.endsWith("p"):
      # 1080p, 720p, etc.
      var numeric = true
      for ch in lower[0 ..< lower.high]:
        if ch < '0' or ch > '9':
          numeric = false
          break
      if numeric:
        return true
    return false

  var tokens = newSeq[string]()
  for token in base.split():
    if token.len == 0:
      continue

    if looksLikeYear(token):
      break

    if looksLikeQuality(token):
      break

    tokens.add(token)

  if tokens.len == 0:
    return base

  return tokens.join(" ")
