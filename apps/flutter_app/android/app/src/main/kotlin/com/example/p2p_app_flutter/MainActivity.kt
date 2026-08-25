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
                "getApkPath" -> {
                    result.success(packageCodePath)
                }
                else -> result.notImplemented()
            }
        }
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
