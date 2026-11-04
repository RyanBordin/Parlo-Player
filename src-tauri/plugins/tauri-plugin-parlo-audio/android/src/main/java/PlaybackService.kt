package com.ryanbordin.parloplayer.audio

import android.content.Intent
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

/**
 * Owns the ExoPlayer and its MediaSession. As a MediaSessionService it keeps
 * playing with the screen off and shows the system media notification;
 * `AudioPlugin` drives it through a MediaController.
 */
class PlaybackService : MediaSessionService() {
  companion object {
    /**
     * Set by `AudioPlugin`. The queue lives in the app, so next/previous from
     * the notification, lock screen and headset buttons go there instead of
     * moving inside ExoPlayer's short "current + next" playlist.
     */
    @Volatile
    var onSkip: ((next: Boolean) -> Unit)? = null
  }

  private var session: MediaSession? = null

  override fun onCreate() {
    super.onCreate()
    val music = AudioAttributes.Builder()
      .setUsage(C.USAGE_MEDIA)
      .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
      .build()
    val player = ExoPlayer.Builder(this)
      // Pauses for calls and other apps' audio, and when headphones are unplugged.
      .setAudioAttributes(music, true)
      .setHandleAudioBecomingNoisy(true)
      // Songs stream from the server: keep the CPU and Wi-Fi awake while playing.
      .setWakeMode(C.WAKE_MODE_NETWORK)
      .build()
    session = MediaSession.Builder(this, QueuePlayer(player)).build()
  }

  /** Sends next/previous to the app's queue; everything else goes to ExoPlayer. */
  private class QueuePlayer(player: Player) : ForwardingPlayer(player) {
    // Shown even with no next song in ExoPlayer's playlist: the queue may have one.
    private val skips = Player.Commands.Builder()
      .addAll(
        Player.COMMAND_SEEK_TO_NEXT,
        Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM,
        Player.COMMAND_SEEK_TO_PREVIOUS,
        Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
      )
      .build()

    override fun getAvailableCommands(): Player.Commands =
      super.getAvailableCommands().buildUpon().addAll(skips).build()

    override fun isCommandAvailable(command: Int): Boolean =
      skips.contains(command) || super.isCommandAvailable(command)

    override fun seekToNext() = skip(true)
    override fun seekToNextMediaItem() = skip(true)
    override fun seekToPrevious() = skip(false)
    override fun seekToPreviousMediaItem() = skip(false)

    private fun skip(next: Boolean) {
      onSkip?.invoke(next)
    }
  }

  override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

  // The queue lives in the app: when the app is swiped away, the music stops with it.
  override fun onTaskRemoved(rootIntent: Intent?) {
    session?.player?.let {
      it.stop()
      it.clearMediaItems()
    }
    stopSelf()
  }

  override fun onDestroy() {
    session?.run {
      player.release()
      release()
    }
    session = null
    super.onDestroy()
  }
}
