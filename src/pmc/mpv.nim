import std/[asyncdispatch, json, asyncnet, net, os, options]

const 
  SOCKET_PATH = "/tmp/mpv-socket"
  mpvSocket: AsyncSocket

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
    status: PlaybackStatus
    filename: string
    position: float
    should_stop: bool

# State manager
var currentState* = MpvState(status: Started, filename: "", position: 0, should_stop: false)

proc getState*(): MpvState =
  return currentState

proc updateState(status: PlaybackStatus = currentState.status,
                 filename: string = currentState.filename,
                 position: float = currentState.position) =
  currentState = MpvState(status: status, filename: filename, position: position)


proc sendCommand(cmd: JsonNode): Future[JsonNode] {.async.} =
  let msg = $cmd & "\n"
  await mpvSocket.send(msg)
  let response = await mpvSocket.recvLine()
  if response.len == 0:
    raise newException(ValueError, "Empty response from MPV")
  result = parseJson(response)

proc get_property(prop: string): Future[JsonNode] {.async.} =
  let cmd = %* { "command": ["get_property", prop] }
  return await sendCommand(cmd)

proc observe_property(name: string, id: int) {.async.} =
  let cmd = %* { "command": ["observe_property", id, name] }
  discard await sendCommand(cmd)

proc seek(pos: string) {.async.} =
  let cmd = %* { "command": ["seek", pos, "absolute-percent"] }
  discard await sendCommand(cmd)

proc user_quit() {.async.} =
  let cmd = %* { "command": ["quit"] }
  discard await sendCommand(cmd)

proc user_stop() {.async.} =
  let cmd = %* { "command": ["stop"] }
  discard await sendCommand(cmd)

proc mpv_start_monitoring() {.async.} = 
  while true:
    let line = await mpvSocket.recvLine()
    if line.len == 0:
      echo "Mpv socket closed"
      quit(1)

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
            let msg == args[0].getStr()
            if msg == "pmc-quit":
              updateState(status = RequestedQuit)
        else:
          discard
    except CatchableError as e:
      echo "Failed to parse line ", line, " error: ", e.msg


proc spawn_mpv*() =
  discard

proc mpv_init*() {.async.} =
  mpvSocket = newAsyncSocket(Domain.AF_UNIX, SockType.SOCK_STREAM, Protocol.IPPROTO_IP)
  await mpvSocket.connectUnix(SOCKET_PATH)

proc play_file*(name: string) {.async.} =
  let cmd = %* { "command": ["loadfile", name, "replace"] }
  discard await sendCommand(cmd)

proc buildPlaylist*(files: seq[string]) {.async.} =
  for f in files:
    let cmd = %* { "command": ["loadfile", f, "append"] }
    discard await sendCommand(cmd)