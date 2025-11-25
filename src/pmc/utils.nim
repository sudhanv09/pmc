import std/[nre, strutils, random]

const defaultAlphabet = "_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"

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
