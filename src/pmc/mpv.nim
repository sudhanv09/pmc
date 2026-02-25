import std/[asyncdispatch, json, asyncnet, net, options, osproc]

var mpvSocket: AsyncSocket = nil
const 
  SOCKET_PATH = "/tmp/mpv-socket"

type
  MpvResponse = object
    data: string
    request_id: int
    error: string

  PlaybackStatus* = enum
    Started,
    Stopped,
    Resumed,
    Paused,
    Completed,
    FileEnded,
    Exited,
    RequestedNext,
    RequestedPrev, 
    RequestedQuit

  MpvState* = ref object
    status*: PlaybackStatus
    filename*: string
    position*: float
    should_stop*: bool

# State manager
var currentState* = MpvState(status: Started, filename: "", position: 0, should_stop: false)

proc getState*(): MpvState =
  return currentState

proc updateState*(status: PlaybackStatus = currentState.status,
                 filename: string = currentState.filename,
                 position: float = currentState.position) =
  currentState.status = status
  currentState.filename = filename
  currentState.position = position

proc sendCommand(cmd: JsonNode) {.async.} =
  let msg = $cmd & "\n"
  await mpvSocket.send(msg)

proc observe_property(name: string, id: int) {.async.} =
  let cmd = %* { "command": ["observe_property", id, name] }
  await sendCommand(cmd)

proc seek*(pos: string) {.async.} =
  let cmd = %* { "command": ["seek", pos, "absolute-percent"] }
  await sendCommand(cmd)

proc user_quit*() {.async.} =
  let cmd = %* { "command": ["quit"] }
  await sendCommand(cmd)

proc user_stop() {.async.} =
  let cmd = %* { "command": ["stop"] }
  await sendCommand(cmd)

proc mpv_start_monitoring*() {.async.} = 
  await observe_property("pause", 1)
  await observe_property("percent-pos", 2)
  await observe_property("eof-reached", 3)

  while true:
    let line = await mpvSocket.recvLine()
    if line.len == 0:
      echo "Mpv socket closed"
      updateState(status = Exited)
      break

    try:
      let evt = parseJson(line)
      if evt.hasKey("event"):
        let ev = evt["event"].getStr()
        case ev
        of "property-change":
          let name = evt["name"].getStr()
          if name == "pause":
            let paused = evt["data"].getBool(false)
            if paused:
              updateState(status = Paused)
            else:
              updateState(status = Resumed)
          elif name == "percent-pos":
            let pos = evt["data"].getFloat()
            updateState(position = pos)
            if pos >= 98:
              updateState(status = Completed, position = pos)
          elif name == "eof-reached":
            if evt["data"].getBool(false):
              if getState().status == Started:
                updateState(status = FileEnded, position = 100)
        of "shutdown":
          updateState(status = Exited)
        of "end-file":
          updateState(status = FileEnded)
        of "playlist-next":
          updateState(status = RequestedNext)
        of "playlist-prev":
          updateState(status = RequestedPrev)
        of "client-message":
          let args = evt["args"]
          if args.kind == JArray and args.len > 0:
            let msg = args[0].getStr()
            if msg == "pmc-quit":
              updateState(status = RequestedQuit)
        else:
          discard
      elif evt.hasKey("request_id") and evt.hasKey("error"):
        let err = evt["error"].getStr()
        if err != "success":
          echo "MPV command failed: ", line
    except CatchableError as e:
      echo "Failed to parse line ", line, " error: ", e.msg


proc spawn_mpv*(socket_path: string = SOCKET_PATH): Process = 
  var mpv_args: seq[string] = @[
    "--input-ipc-server=" & socket_path,
    "--idle=yes",
    "--force-window",
  ]
  return startProcess("mpv", args = mpv_args, options = {poUsePath, poParentStreams})
  
proc mpv_init*() {.async.} =
  mpvSocket = newAsyncSocket(Domain.AF_UNIX, SockType.SOCK_STREAM, Protocol.IPPROTO_IP)
  await mpvSocket.connectUnix(SOCKET_PATH)

proc play_file*(name: string) {.async.} =
  let cmd = %* { "command": ["loadfile", name, "replace"] }
  await sendCommand(cmd)

proc buildPlaylist*(files: seq[string]) {.async.} =
  for f in files:
    let cmd = %* { "command": ["loadfile", f, "append"] }
    await sendCommand(cmd)

proc set_playlist_pos*(index: int) {.async.} =
  let cmd = %* { "command": ["set_property", "playlist-pos", index] }
  await sendCommand(cmd)
