package com.example.p2p_app_flutter

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.media.RingtoneManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

/**
 * Native half of the notification delivery adapter (US-027 sounds, US-028
 * system notifications). Rust owns the policy, the text and the two channel
 * flags; this object only maps the two MethodChannel calls onto Android APIs.
 * Keeping it in its own file leaves [MainActivity] a thin wiring shell.
 */
object MessageNotifications {
    const val CHANNEL = "com.example.p2p_app_flutter/notifications"
    const val SOUND = "playSound"
    const val SHOW = "show"

    private const val CHANNEL_ID = "p2p_messages"
    private const val NOTIFICATION_ID = 2

    /** Handle one call from Dart. Every branch returns so no call hangs. */
    fun handle(call: MethodCall, context: Context, result: MethodChannel.Result) {
        when (call.method) {
            SOUND -> {
                playSound(context)
                result.success(null)
            }
            SHOW -> {
                val title = call.argument<String>("title") ?: "New message"
                val body = call.argument<String>("body") ?: ""
                show(context, title, body)
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    /** Play the platform's default short notification sound once. */
    private fun playSound(context: Context) {
        val uri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
        RingtoneManager.getRingtone(context.applicationContext, uri)?.play()
    }

    /** Post a high-importance message notification tapping back into the app. */
    private fun show(context: Context, title: String, body: String) {
        createChannel(context)
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            context,
            0,
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_notify_chat)
            .setContentTitle(title)
            .setContentText(body)
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()
        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
    }

    /** Android 8+ requires an explicit high-importance channel. Idempotent. */
    private fun createChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Messages",
            NotificationManager.IMPORTANCE_HIGH,
        ).apply { description = "Incoming chat messages" }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }
}
