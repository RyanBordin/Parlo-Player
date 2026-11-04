package com.ryanbordin.parloplayer

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // Back is handled below instead of by Tauri, which finishes the activity
  // on the first page: that ends the process and stops the music.
  override val handleBackNavigation: Boolean = false

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        // Back goes to the previous page of the app; on the first page the
        // app goes to the background, like other music players, and keeps
        // playing. The page decides (window.parloBack): WebView.canGoBack()
        // skips history entries added without a tap, so it can say there is
        // nowhere to go when there is.
        webView.evaluateJavascript("window.parloBack ? window.parloBack() : false") { result ->
          if (result != "true") moveTaskToBack(true)
        }
      }
    })
  }
}
