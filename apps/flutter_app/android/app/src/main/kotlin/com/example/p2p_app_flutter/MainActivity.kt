package com.example.p2p_app_flutter

import android.content.Intent
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {

    private val CHANNEL = "com.example.p2p_app_flutter/service"
    private var pendingDbPath: String? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            CHANNEL,
        ).setMethodCallHandler { call, result ->
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
}
