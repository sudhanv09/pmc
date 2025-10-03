import std/[nre, strutils]

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

echo guessEpisode("Cartoon Ep05.mp4")
