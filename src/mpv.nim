import std/[asyncdispatch, json, asyncnet, net, os]

const SOCKET_PATH* = "/tmp/mpv-socket"

var mpvSocket: AsyncSocket

type
  MpvResponse* = object
    data: string
    request_id: int
    error: string

  MpvState* = ref object
    playing: bool
    filename: string
    paused: bool
    position: float
    should_stop: bool


proc sendCommand(cmd: seq[string]): Future[JsonNode] {.async.} =
  let jsonCmd = %* {"command": cmd}
  let msg = $jsonCmd & "\n"
  await mpvSocket.send(msg)
  let response = await mpvSocket.recvLine()
  if response.len == 0:
    raise newException(ValueError, "Empty response from MPV")
  result = parseJson(response)

proc get_property(prop: string): Future[JsonNode] {.async.} =
  await sendCommand(@["get_property", prop])

proc seek(pos: string) {.async.} =
  await sendCommand(@["seek", pos, "absolute-percent"])

proc user_quit() {.async.} =
  await sendCommand(@["quit"])

proc user_stop() {.async.} =
  await sendCommand(@["stop"])

proc mpv_start_monitoring(state: MpvState) {.async.} = 
  while true:
    let line = await mpvSocket.recvLine()
    if line.len == 0:
      echo "Mpv socket closed"
      quit(1)

    try:

    except CatchableError as e:
      echo "Failed to parse line", line, " error: ", e

proc spawn_mpv*() =
  discard

proc mpv_init*() {.async.} =
  mpvSocket = newAsyncSocket(Domain.AF_UNIX, SockType.SOCK_STREAM, Protocol.IPPROTO_IP)
  await mpvSocket.connectUnix(SOCKET_PATH)

proc play_file*(name: string) {.async.} =
  await sendCommand(@["loadfile", name, "replace"])
