package com.example.p2p_app_flutter

import android.content.Intent
import androidx.core.content.FileProvider
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.io.File

class MainActivity : FlutterActivity() {

    private val CHANNEL = "com.example.p2p_app_flutter/service"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // Notification delivery adapter (US-027/US-028): policy and text are
        // Rust-owned; this channel only maps the two flags onto Android APIs.
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            MessageNotifications.CHANNEL,
        ).setMethodCallHandler { call, result ->
            MessageNotifications.handle(call, this, result)
        }

        // Device-location adapter: Rust owns validation and sharing policy; this
        // channel only returns one raw fix, one raw accelerometer motion state
        // and the Android runtime permission answer.
        LocationSensor.attach(this)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            LocationSensor.CHANNEL,
        ).setMethodCallHandler { call, result ->
            LocationSensor.handle(call, this, result)
        }

        // Call-media device adapter (US-044): Rust owns framing/jitter/mute,
        // these channels only move PCM to and from AudioRecord/AudioTrack.
        val audioEvents = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            CallAudio.EVENTS,
        )
        CallAudio.attach(this, audioEvents)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            CallAudio.CHANNEL,
        ).setMethodCallHandler { call, result ->
            CallAudio.handle(call, result)
        }

        val channel = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            CHANNEL,
        )

        // Share the channel with the service so it can call back into Dart
        P2pForegroundService.setMethodChannel(channel)

        channel.setMethodCallHandler { call, result ->
            when (call.method) {
                "startService" -> {
                    val dbPath = call.argument<String>("dbPath") ?: "p2p.db"
                    startP2pService(dbPath)
                    result.success(true)
                }
                "stopService" -> {
                    stopP2pService()
                    result.success(true)
                }
                "isServiceRunning" -> {
                    result.success(P2pForegroundService.isRunning())
                }
                "shareApk" -> {
                    shareApk()
                    result.success(true)
                }
                else -> result.notImplemented()
            }
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        CallAudio.onRequestPermissionsResult(requestCode, grantResults)
        LocationSensor.onRequestPermissionsResult(requestCode, grantResults)
    }

    private fun startP2pService(dbPath: String) {
        val intent = Intent(this, P2pForegroundService::class.java).apply {
            action = P2pForegroundService.ACTION_START
            putExtra(P2pForegroundService.EXTRA_DB_PATH, dbPath)
        }
        startForegroundService(intent)
    }

    private fun stopP2pService() {
        val intent = Intent(this, P2pForegroundService::class.java).apply {
            action = P2pForegroundService.ACTION_STOP
        }
        startService(intent)
    }

    private fun shareApk() {
        // The installed APK is exposed by the system as `base.apk`; copy it to a
        // cache file named `p2p_app.apk` so the share sheet offers a sensible
        // filename instead of the internal `base.apk`.
        val source = File(packageCodePath)
        val target = File(cacheDir, "p2p_app.apk")
        source.copyTo(target, overwrite = true)
        val uri = FileProvider.getUriForFile(
            this,
            "${packageName}.fileprovider",
            target,
        )
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "application/vnd.android.package-archive"
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        startActivity(Intent.createChooser(intent, "Share P2P Chat"))
    }
}
