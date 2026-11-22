package com.example.p2p_app_flutter

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaRecorder
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.ConcurrentLinkedQueue
import kotlin.concurrent.thread

/**
 * Native half of the Android call-media device adapter (US-044).
 *
 * Rust owns the media session — μ-law framing, sequence numbers, jitter buffer
 * and mute. This object only moves 16 kHz mono i16 PCM between `AudioRecord` /
 * `AudioTrack` and Dart, at exactly the frame size Rust expects, so no resampler
 * is needed on this platform.
 *
 * Capture is activity-scoped: the call screen stops the adapter on disposal and
 * there is no microphone foreground-service type here, so backgrounding the app
 * ends capture rather than silently keeping the mic open.
 */
object CallAudio {
    /** Dart -> native command channel. */
    const val CHANNEL = "com.example.p2p_app_flutter/call_audio"
    const val START = "start"
    const val PLAY = "play"
    const val STOP = "stop"

    /** Native -> Dart event channel, kept separate so the two MethodChannel
     * handlers never compete for the same binary-messenger slot. */
    const val EVENTS = "com.example.p2p_app_flutter/call_audio_events"

    /** Native -> Dart event carrying one captured frame as a ByteArray. */
    const val PCM = "pcm"

    const val SAMPLE_RATE = 16000
    const val FRAME_SAMPLES = 320
    const val FRAME_BYTES = FRAME_SAMPLES * 2

    const val PERMISSION_REQUEST_CODE = 4412
    const val PERMISSION_ERROR = "Mic unavailable (allow microphone access)"
    const val DEVICE_ERROR = "Mic unavailable (audio device error)"

    private var activity: Activity? = null
    private var eventChannel: MethodChannel? = null
    private var pendingResult: MethodChannel.Result? = null
    private var lastError: String? = null

    private var recorder: AudioRecord? = null
    private var player: AudioTrack? = null
    @Volatile private var running = false
    private val playbackQueue = ConcurrentLinkedQueue<ByteArray>()
    private val mainHandler = Handler(Looper.getMainLooper())

    /** Called once from [MainActivity.configureFlutterEngine]. */
    fun attach(host: Activity, events: MethodChannel) {
        activity = host
        eventChannel = events
    }

    /** Handle one Dart -> native call. Every branch replies so no call hangs. */
    fun handle(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            START -> start(result)
            PLAY -> play(call, result)
            STOP -> stop(result)
            else -> result.notImplemented()
        }
    }

    /** Forward the runtime permission answer from [MainActivity]. */
    fun onRequestPermissionsResult(requestCode: Int, grantResults: IntArray) {
        if (requestCode != PERMISSION_REQUEST_CODE) return
        val result = pendingResult ?: return
        pendingResult = null
        val granted = grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED
        begin(result, capture = granted)
    }

    private fun start(result: MethodChannel.Result) {
        if (running) {
            result.success(lastError)
            return
        }
        val host = activity
        if (host == null) {
            result.error("UNAVAILABLE", "Activity not attached", null)
            return
        }
        if (ContextCompat.checkSelfPermission(
                host,
                Manifest.permission.RECORD_AUDIO,
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            // Defer the reply until the permission dialog is answered. stop()
            // replies to this pending result too, so a hang-up mid-prompt never
            // leaves Dart's start() future unresolved.
            pendingResult = result
            ActivityCompat.requestPermissions(
                host,
                arrayOf(Manifest.permission.RECORD_AUDIO),
                PERMISSION_REQUEST_CODE,
            )
            return
        }
        begin(result, capture = true)
    }

    /** Open playback (always) and capture (when permitted), then start the worker. */
    private fun begin(result: MethodChannel.Result, capture: Boolean) {
        val track = try {
            createTrack()
        } catch (e: Exception) {
            result.error("AUDIO", e.message ?: "AudioTrack init failed", null)
            return
        }

        var rec: AudioRecord? = null
        // No capture only happens when permission was denied; a granted but
        // failed recorder is reported as DEVICE_ERROR below.
        var error: String? = if (capture) null else PERMISSION_ERROR
        if (capture) {
            rec = try {
                createRecorder()
            } catch (e: Exception) {
                error = DEVICE_ERROR
                null
            }
            if (rec?.state != AudioRecord.STATE_INITIALIZED) {
                rec?.release()
                rec = null
                error = DEVICE_ERROR
            }
        }

        recorder = rec
        player = track
        lastError = error
        running = true
        playbackQueue.clear()
        track.play()
        rec?.startRecording()
        val events = eventChannel
        val activeRecorder = rec
        thread(name = "p2p-call-audio") {
            workerLoop(track, activeRecorder, events)
        }
        result.success(error)
    }

    private fun play(call: MethodCall, result: MethodChannel.Result) {
        val bytes = call.arguments as? ByteArray
        if (bytes != null && running) playbackQueue.add(bytes)
        result.success(null)
    }

    private fun stop(result: MethodChannel.Result) {
        // A permission prompt can still be open; reply so Dart's start() settles.
        pendingResult?.success(null)
        pendingResult = null
        running = false
        playbackQueue.clear()

        recorder?.let { rec ->
            try {
                rec.stop()
            } catch (_: IllegalStateException) {
            }
            rec.release()
        }
        player?.let { track ->
            try {
                track.stop()
            } catch (_: IllegalStateException) {
            }
            track.release()
        }
        recorder = null
        player = null
        lastError = null
        result.success(null)
    }

    /** One worker drains queued playback and reads capture. 20 ms per cycle. */
    private fun workerLoop(track: AudioTrack, rec: AudioRecord?, events: MethodChannel?) {
        val buffer = ByteArray(FRAME_BYTES)
        while (running) {
            while (true) {
                val out = playbackQueue.poll() ?: break
                writeTrack(track, out)
            }
            if (rec == null) {
                Thread.sleep(FRAME_SAMPLES * 1000L / SAMPLE_RATE)
                continue
            }
            val read = rec.read(buffer, 0, FRAME_BYTES)
            if (read > 0 && running && events != null) {
                val frame = buffer.copyOf(read)
                // MethodChannel callbacks must run on the platform thread.
                mainHandler.post { events.invokeMethod(PCM, frame) }
            }
        }
    }

    private fun createTrack(): AudioTrack {
        val bufferSize = maxOf(
            AudioTrack.getMinBufferSize(
                SAMPLE_RATE,
                AudioFormat.CHANNEL_OUT_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
            ),
            FRAME_BYTES * 4,
        )
        return AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build(),
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(SAMPLE_RATE)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build(),
            )
            .setBufferSizeInBytes(bufferSize)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()
    }

    @Suppress("DEPRECATION") // legacy constructor is the minSdk 24 fallback below
    private fun createRecorder(): AudioRecord {
        val bufferSize = maxOf(
            AudioRecord.getMinBufferSize(
                SAMPLE_RATE,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
            ),
            FRAME_BYTES * 2,
        )
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            AudioRecord.Builder()
                .setAudioSource(MediaRecorder.AudioSource.MIC)
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(SAMPLE_RATE)
                        .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
                        .build(),
                )
                .setBufferSizeInBytes(bufferSize)
                .build()
        } else {
            // Legacy constructor kept for minSdk 24; identical format at 16 kHz.
            AudioRecord(
                MediaRecorder.AudioSource.MIC,
                SAMPLE_RATE,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
                bufferSize,
            )
        }
    }

    @Suppress("DEPRECATION") // legacy write() is the pre-33 fallback below
    private fun writeTrack(track: AudioTrack, bytes: ByteArray) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            track.write(bytes, 0, bytes.size, AudioTrack.WRITE_BLOCKING)
        } else {
            track.write(bytes, 0, bytes.size)
        }
    }
}
