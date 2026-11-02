package com.ryanbordin.parloplayer.audio

import android.content.Intent
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

/**
 * Owns the ExoPlayer and its MediaSession. As a MediaSessionService it keeps
 * playing with the screen off and shows the system media notification;
 * `AudioPlugin` drives it through a MediaController.
 */
class PlaybackService : MediaSessionService() {
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
    session = MediaSession.Builder(this, player).build()
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
