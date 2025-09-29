import std/[asyncdispatch, json, asyncnet, net, os, options]

const 
  SOCKET_PATH* = "/tmp/mpv-socket"
  mpvSocket: AsyncSocket

type
  MpvResponse* = object
    data: string
    request_id: int
    error: string

  MpvEvent* = object
    event: string
    id: int
    name: string
    data: JsonNode

  MpvState* = ref object
    playing: bool
    filename: string
    paused: bool
    position: float
    should_stop: bool


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
        let mpv_event = evt.to(MpvEvent)
        case mpv_event.event:
          of "property-change":
            case mpv_event.name:
              of "pause":
                echo "paused playback"
              of "percent-pos":
                echo mpv_event.data
              of "eof-reached":
                echo "playback finished"
              # else:
              #   echo "here"
          else:
            echo mpv_event.event

    except CatchableError as e:
      echo "Failed to parse line", line, " error: ", e.msg

proc spawn_mpv*() =
  discard

proc mpv_init*() {.async.} =
  mpvSocket = newAsyncSocket(Domain.AF_UNIX, SockType.SOCK_STREAM, Protocol.IPPROTO_IP)
  await mpvSocket.connectUnix(SOCKET_PATH)

proc play_file*(name: string) {.async.} =
  let cmd = %* { "command": ["loadfile", name, "replace"] }
  discard await sendCommand(cmd)


  await sendCommand(@["loadfile", name, "replace"])
