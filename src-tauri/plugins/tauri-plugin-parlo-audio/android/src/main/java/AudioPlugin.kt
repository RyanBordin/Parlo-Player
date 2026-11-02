package com.ryanbordin.parloplayer.audio

import android.app.Activity
import android.content.ComponentName
import android.os.Handler
import android.os.Looper
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

private const val POSITION_INTERVAL_MS = 250L

@InvokeArg
class SongArgs {
  // Has the server credentials: never logged.
  lateinit var url: String
  var title: String = ""
  var artist: String? = null
}

@InvokeArg
class PlayArgs {
  lateinit var url: String
  var title: String = ""
  var artist: String? = null
  var startMs: Long = 0
  var paused: Boolean = false
  // Queued right after, so ExoPlayer moves to it without a gap.
  var next: SongArgs? = null
}

@InvokeArg
class SetNextArgs {
  var next: SongArgs? = null
}

@InvokeArg
class SeekArgs {
  var ms: Long = 0
}

@InvokeArg
class VolumeArgs {
  var volume: Double = 100.0
}

@InvokeArg
class EventHandlerArgs {
  lateinit var channel: Channel
}

/**
 * Commands from the app's Rust code to the player in `PlaybackService`, and
 * the player's events back through a Channel.
 *
 * A MediaController must be used from the thread it was built on, so every
 * call is posted to the main looper. Commands resolve as soon as they are
 * queued; what the player then does arrives as events.
 */
@TauriPlugin
class AudioPlugin(private val activity: Activity) : Plugin(activity) {
  private val main = Handler(Looper.getMainLooper())
  private var controller: MediaController? = null
  // Commands sent before the controller is connected run once it is.
  private val pending = ArrayDeque<(MediaController) -> Unit>()
  private var channel: Channel? = null
  private var lastState: String? = null
  // Each queued song gets its own id, so a real move to the next song can be
  // told from a playlist change: the controller repeats the last transition
  // reason (AUTO) when items are removed or added around the current one.
  private var nextItemId = 0L
  private var currentItemId: String? = null

  private val positionTicker = object : Runnable {
    override fun run() {
      val player = controller ?: return
      sendPosition(player)
      if (player.isPlaying) main.postDelayed(this, POSITION_INTERVAL_MS)
    }
  }

  private val listener = object : Player.Listener {
    override fun onEvents(player: Player, events: Player.Events) {
      if (events.containsAny(
          Player.EVENT_PLAYBACK_STATE_CHANGED,
          Player.EVENT_PLAY_WHEN_READY_CHANGED,
          Player.EVENT_IS_PLAYING_CHANGED,
        )
      ) {
        sendState(player)
      }
      if (events.contains(Player.EVENT_IS_PLAYING_CHANGED) && player.isPlaying) {
        main.removeCallbacks(positionTicker)
        main.post(positionTicker)
      }
    }

    // ExoPlayer moved to the queued next song by itself: keep the playlist
    // at "current + next" (the app queues the following one) and tell the app.
    override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
      if (reason != Player.MEDIA_ITEM_TRANSITION_REASON_AUTO) return
      val id = mediaItem?.mediaId ?: return
      if (id == currentItemId) return
      currentItemId = id
      // First the switch, then the new song's position, as mpv reports them.
      send(JSObject().put("kind", "advanced"))
      controller?.let { sendPosition(it) }
      main.post {
        controller?.let { player ->
          if (player.currentMediaItemIndex > 0) player.removeMediaItems(0, player.currentMediaItemIndex)
        }
      }
    }

    override fun onPositionDiscontinuity(
      oldPosition: Player.PositionInfo,
      newPosition: Player.PositionInfo,
      reason: Int,
    ) {
      if (reason == Player.DISCONTINUITY_REASON_SEEK) {
        send(JSObject().put("kind", "seeked"))
        controller?.let { sendPosition(it) }
      }
    }

    override fun onPlayerError(error: PlaybackException) {
      // The error code name and the generic message, never the URL.
      sendError("${error.errorCodeName}: ${error.message}")
    }
  }

  override fun load(webView: WebView) {
    PlaybackService.onSkip = { next ->
      main.post { send(JSObject().put("kind", "skip").put("next", next)) }
    }
    main.post { connect() }
  }

  override fun onDestroy(activity: AppCompatActivity) {
    PlaybackService.onSkip = null
    main.removeCallbacks(positionTicker)
    controller?.release()
    controller = null
  }

  private fun connect() {
    val token = SessionToken(activity, ComponentName(activity, PlaybackService::class.java))
    val future = MediaController.Builder(activity, token).buildAsync()
    future.addListener({
      val connected = try {
        future.get()
      } catch (e: Exception) {
        sendError("could not connect to the playback service: ${e.message}")
        return@addListener
      }
      connected.addListener(listener)
      controller = connected
      while (pending.isNotEmpty()) run(pending.removeFirst(), connected)
    }, ContextCompat.getMainExecutor(activity))
  }

  @Command
  fun play(invoke: Invoke) {
    val args = invoke.parseArgs(PlayArgs::class.java)
    val items = listOfNotNull(mediaItem(args.url, args.title, args.artist), args.next?.let(::mediaItem))
    onPlayer(invoke) {
      lastState = null
      currentItemId = items.first().mediaId
      it.setMediaItems(items, 0, args.startMs.coerceAtLeast(0))
      it.playWhenReady = !args.paused
      it.prepare()
    }
  }

  /** Replaces whatever is queued after the current song. */
  @Command
  fun setNext(invoke: Invoke) {
    val args = invoke.parseArgs(SetNextArgs::class.java)
    val next = args.next?.let(::mediaItem)
    onPlayer(invoke) {
      if (it.mediaItemCount == 0) return@onPlayer
      val after = it.currentMediaItemIndex + 1
      if (after < it.mediaItemCount) it.removeMediaItems(after, it.mediaItemCount)
      if (next != null) it.addMediaItem(next)
    }
  }

  @Command
  fun togglePause(invoke: Invoke) {
    onPlayer(invoke) {
      if (it.mediaItemCount == 0) return@onPlayer
      if (it.playWhenReady) it.pause() else it.play()
    }
  }

  @Command
  fun seek(invoke: Invoke) {
    val args = invoke.parseArgs(SeekArgs::class.java)
    onPlayer(invoke) {
      if (it.mediaItemCount > 0) it.seekTo(args.ms.coerceAtLeast(0))
    }
  }

  @Command
  fun stop(invoke: Invoke) {
    // Stops and forgets the queued next song too.
    onPlayer(invoke) {
      it.stop()
      it.clearMediaItems()
    }
  }

  @Command
  fun setVolume(invoke: Invoke) {
    val args = invoke.parseArgs(VolumeArgs::class.java)
    onPlayer(invoke) {
      it.volume = (args.volume / 100.0).toFloat().coerceIn(0f, 1f)
    }
  }

  @Command
  fun setEventHandler(invoke: Invoke) {
    val args = invoke.parseArgs(EventHandlerArgs::class.java)
    main.post { channel = args.channel }
    invoke.resolve()
  }

  private fun mediaItem(song: SongArgs): MediaItem = mediaItem(song.url, song.title, song.artist)

  private fun mediaItem(url: String, title: String, artist: String?): MediaItem {
    // The title and artist are what the system media controls show.
    val metadata = MediaMetadata.Builder().setTitle(title).setArtist(artist).build()
    return MediaItem.Builder()
      .setMediaId("item-${nextItemId++}")
      .setUri(url)
      .setMediaMetadata(metadata)
      .build()
  }

  private fun onPlayer(invoke: Invoke, action: (MediaController) -> Unit) {
    main.post {
      val player = controller
      if (player != null) run(action, player) else pending.addLast(action)
    }
    invoke.resolve()
  }

  private fun run(action: (MediaController) -> Unit, player: MediaController) {
    try {
      action(player)
    } catch (e: Exception) {
      sendError("player command failed: ${e.message}")
    }
  }

  private fun sendState(player: Player) {
    val state = when (player.playbackState) {
      Player.STATE_READY -> if (player.playWhenReady) "playing" else "paused"
      Player.STATE_BUFFERING -> if (player.playWhenReady) "loading" else "paused"
      Player.STATE_ENDED -> "ended"
      else -> "idle"
    }
    if (state == lastState) return
    lastState = state
    send(JSObject().put("kind", "state").put("state", state))
  }

  private fun sendPosition(player: Player) {
    val event = JSObject()
      .put("kind", "position")
      .put("positionMs", player.currentPosition.coerceAtLeast(0))
    if (player.duration != C.TIME_UNSET) event.put("durationMs", player.duration)
    send(event)
  }

  private fun sendError(message: String) {
    send(JSObject().put("kind", "error").put("message", message))
  }

  private fun send(event: JSObject) {
    channel?.send(event)
  }
}
