import std/[strutils]

# pretty print a list of items in a grid
proc pprint*(items: seq[string], cols: int = 1) =
  if items.len == 0:
    return

  let rows = (items.len + cols - 1) div cols

  var colWidths = newSeq[int](cols)
  for c in 0 ..< cols:
    var maxW = 0
    for r in 0 ..< rows:
      let idx = r * cols + c
      if idx < items.len:
        maxW = max(maxW, items[idx].len)
    colWidths[c] = maxW

  for r in 0 ..< rows:
    for c in 0 ..< cols:
      let idx = r * cols + c
      if idx < items.len:
        let it = items[idx]
        stdout.write it
        if c < cols - 1:
          stdout.write repeat(' ', colWidths[c] - it.len + 2)
    echo ""
    
