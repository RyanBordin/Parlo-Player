package com.ryanbordin.parloplayer.credentials

import android.app.Activity
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private const val KEYSTORE = "AndroidKeyStore"
private const val KEY_ALIAS = "parlo-credentials"
private const val TRANSFORMATION = "AES/GCM/NoPadding"
private const val TAG_BITS = 128

// Also named in the app's backup rules (res/xml): it is left out of backups,
// since the Keystore key that decrypts it never leaves the device.
const val PREFS_FILE = "parlo_credentials"

@InvokeArg
class AccountArgs {
  lateinit var account: String
}

@InvokeArg
class SetPasswordArgs {
  lateinit var account: String
  // Never logged, and never part of an error message.
  lateinit var password: String
}

/**
 * Keeps passwords encrypted with an AES key that lives in the Android
 * Keystore (the key never leaves it). Only "IV:ciphertext" in Base64 is
 * written, to a private SharedPreferences file, one entry per account
 * ("{username}@{server_url}", as on desktop).
 */
@TauriPlugin
class CredentialsPlugin(private val activity: Activity) : Plugin(activity) {
  private val prefs by lazy { activity.getSharedPreferences(PREFS_FILE, Context.MODE_PRIVATE) }

  @Command
  fun setPassword(invoke: Invoke) {
    val args = invoke.parseArgs(SetPasswordArgs::class.java)
    try {
      val cipher = Cipher.getInstance(TRANSFORMATION)
      cipher.init(Cipher.ENCRYPT_MODE, key())
      val encrypted = cipher.doFinal(args.password.toByteArray(Charsets.UTF_8))
      prefs.edit().putString(args.account, "${encode(cipher.iv)}:${encode(encrypted)}").commit()
      invoke.resolve()
    } catch (e: Exception) {
      invoke.reject("could not save the password (${e.javaClass.simpleName})")
    }
  }

  @Command
  fun getPassword(invoke: Invoke) {
    val args = invoke.parseArgs(AccountArgs::class.java)
    val stored = prefs.getString(args.account, null)
    val result = JSObject()
    if (stored != null) {
      try {
        val (iv, encrypted) = stored.split(":", limit = 2).map { Base64.decode(it, Base64.NO_WRAP) }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, iv))
        result.put("password", String(cipher.doFinal(encrypted), Charsets.UTF_8))
      } catch (e: Exception) {
        // The key was replaced (e.g. after a restore) or the entry is damaged:
        // forget it, so the app simply asks to log in again.
        prefs.edit().remove(args.account).commit()
      }
    }
    invoke.resolve(result)
  }

  /** Succeeds when nothing is saved. */
  @Command
  fun deletePassword(invoke: Invoke) {
    val args = invoke.parseArgs(AccountArgs::class.java)
    prefs.edit().remove(args.account).commit()
    invoke.resolve()
  }

  /** The app's key in the Android Keystore, created on first use. */
  private fun key(): SecretKey {
    val keyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }
    (keyStore.getKey(KEY_ALIAS, null) as? SecretKey)?.let { return it }
    val spec = KeyGenParameterSpec.Builder(
      KEY_ALIAS,
      KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
    )
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
      .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
      .setKeySize(256)
      .build()
    return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
      .apply { init(spec) }
      .generateKey()
  }

  private fun encode(bytes: ByteArray): String = Base64.encodeToString(bytes, Base64.NO_WRAP)
}
