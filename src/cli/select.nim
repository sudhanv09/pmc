import terminal, math

proc select*(options: seq[string], prompt: string = "Select an option:", columns: int = 2): int =
  var selectedIndex = 0  
  let rows = (options.len + columns - 1) div columns
  
  hideCursor()
  
  proc render() =
    for i in 0..<(rows + 1):
      cursorUp(1)
      eraseLine()
    
    echo prompt
    for row in 0..<rows:
      var line = ""
      for col in 0..<columns:
        let idx = row + col * rows
        if idx < options.len:
          let option = options[idx]
          if idx == selectedIndex:
            line.add("\e[1;32m> " & option & "\e[0m")
          else:
            line.add("  " & option)
          
          if col < columns - 1:
            line.add("    ")
      echo line
  
  # Initial render
  echo prompt
  for row in 0..<rows:
    var line = ""
    for col in 0..<columns:
      let idx = row + col * rows
      if idx < options.len:
        let option = options[idx]
        if idx == selectedIndex:
          line.add("\e[1;32m> " & option & "\e[0m")
        else:
          line.add("  " & option)
        
        if col < columns - 1:
          line.add("    ")
    echo line
  
  while true:
    let key = getch()
    
    case ord(key)
    of 13: # Enter
      showCursor()
      return selectedIndex
    of 9: # Tab
      selectedIndex = (selectedIndex + 1) mod options.len
      render()
    of 27: # Escape or arrow keys
      let next = getch()
      if next == '[':
        let arrow = getch()
        case arrow
        of 'A': # Up arrow - move up in same column
          selectedIndex = (selectedIndex - 1 + options.len) mod options.len
          render()
        of 'B': # Down arrow - move down in same column
          selectedIndex = (selectedIndex + 1) mod options.len
          render()
        of 'C': # Right arrow - move to next column
          let currentRow = selectedIndex mod rows
          let currentCol = selectedIndex div rows
          let nextCol = (currentCol + 1) mod columns
          let newIdx = currentRow + nextCol * rows
          if newIdx < options.len:
            selectedIndex = newIdx
          render()
        of 'D': # Left arrow - move to previous column
          let currentRow = selectedIndex mod rows
          let currentCol = selectedIndex div rows
          let prevCol = (currentCol - 1 + columns) mod columns
          let newIdx = currentRow + prevCol * rows
          if newIdx < options.len:
            selectedIndex = newIdx
          render()
        else: discard
    of 3: # Ctrl+C
      showCursor()
      quit(0)
    else:
      discard