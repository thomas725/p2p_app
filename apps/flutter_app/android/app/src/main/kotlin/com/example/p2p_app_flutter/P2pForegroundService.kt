package com.example.p2p_app_flutter

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import io.flutter.plugin.common.MethodChannel

class P2pForegroundService : Service() {

    companion object {
        const val CHANNEL_ID = "p2p_networking"
        const val NOTIFICATION_ID = 1
        const val ACTION_START = "com.example.p2p_app_flutter.ACTION_START"
        const val ACTION_STOP = "com.example.p2p_app_flutter.ACTION_STOP"
        const val EXTRA_DB_PATH = "db_path"
        private var instance: P2pForegroundService? = null
        private var methodChannel: MethodChannel? = null

        fun setMethodChannel(channel: MethodChannel) {
            methodChannel = channel
        }

        fun isRunning(): Boolean = instance != null
    }

    private var multicastLock: WifiManager.MulticastLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                releaseMulticastLock()
                sendStopNetworking()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                return START_NOT_STICKY
            }
            ACTION_START -> {
                val dbPath = intent.getStringExtra(EXTRA_DB_PATH) ?: "p2p.db"
                acquireMulticastLock()
                startForeground(NOTIFICATION_ID, buildNotification("P2P networking active"))
                sendStartNetworking(dbPath)
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        releaseMulticastLock()
        sendStopNetworking()
        instance = null
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "P2P Networking",
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = "Keeps p2p networking alive in the background"
            }
            val nm = getSystemService(NotificationManager::class.java)
            nm.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE,
        )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("P2P Chat")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    private fun acquireMulticastLock() {
        val wm = applicationContext.getSystemService(WIFI_SERVICE) as WifiManager
        multicastLock = wm.createMulticastLock("p2p_mdns").apply {
            setReferenceCounted(true)
            acquire()
        }
    }

    private fun releaseMulticastLock() {
        multicastLock?.release()
        multicastLock = null
    }

    private fun sendStartNetworking(dbPath: String) {
        methodChannel?.invokeMethod("startNetworking", dbPath)
    }

    private fun sendStopNetworking() {
        methodChannel?.invokeMethod("stopNetworking", null)
    }
}
