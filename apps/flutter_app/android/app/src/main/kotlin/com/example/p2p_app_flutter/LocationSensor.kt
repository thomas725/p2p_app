package com.example.p2p_app_flutter

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.lang.reflect.Proxy
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.sqrt

/**
 * Native half of the Android device-location adapter.
 *
 * Rust owns validation, message text, expiry and sharing cadence. This object
 * only returns one raw GPS/network fix and one raw accelerometer motion state:
 * `getLocation` prefers the fused provider when Google Play services happen to
 * be installed (reflected so no new Gradle dependency is needed), otherwise it
 * uses LocationManager with a short fresh-fix window. `isMoving` is a low-power
 * one-shot accelerometer sample and fails open when there is no sensor.
 *
 * The only permission this object owns is the Android runtime prompt: Dart
 * calls `requestPermission` (or `getLocation`, which requests it on demand)
 * and the answer is delivered through [onRequestPermissionsResult]. Policy and
 * error text still stay in Dart/Rust; this side only reports a raw grant.
 */
object LocationSensor {
    const val CHANNEL = "p2p_app/location"
    const val REQUEST_PERMISSION = "requestPermission"
    const val GET_LOCATION = "getLocation"
    const val IS_MOVING = "isMoving"
    const val CLOSE = "close"

    const val PERMISSION_REQUEST_CODE = 4413
    private val LOCATION_PERMISSIONS = arrayOf(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
    )

    /** Gravity-compensated acceleration (m/s²) counted as movement. */
    const val MOTION_THRESHOLD = 0.5f

    private const val GRAVITY_ALPHA = 0.8f
    private const val MOTION_TIMEOUT_MS = 800L
    private const val LOCATION_TIMEOUT_MS = 8000L
    private const val MAX_FIX_AGE_MS = 60_000L

    // Created lazily so pure helpers and JVM unit tests never touch Looper.
    private val mainHandler by lazy { Handler(Looper.getMainLooper()) }

    private var activity: Activity? = null
    private val pendingPermissionReplies = mutableListOf<(Boolean) -> Unit>()

    private var pendingLocation: MethodChannel.Result? = null
    private var pendingMotion: MethodChannel.Result? = null
    private var locationDelivered: AtomicBoolean? = null
    private var motionDelivered: AtomicBoolean? = null
    private var locationManager: LocationManager? = null
    private var locationListener: LocationListener? = null
    private var motionListener: SensorEventListener? = null
    private var motionManager: SensorManager? = null
    private var locationTimeout: Runnable? = null
    private var motionTimeout: Runnable? = null
    private var lastFix: Location? = null

    /** Called once from [MainActivity.configureFlutterEngine]. */
    fun attach(host: Activity) {
        activity = host
    }

    /** Handle one Dart -> native call. Every branch replies so no call hangs. */
    fun handle(call: MethodCall, context: Context?, result: MethodChannel.Result) {
        when (call.method) {
            REQUEST_PERMISSION -> requestPermission(context, result)
            GET_LOCATION -> {
                if (context == null) result.success(null) else getLocation(context, result)
            }
            IS_MOVING -> {
                // No attached activity means no sensor: fail open.
                if (context == null) result.success(true) else isMoving(context, result)
            }
            CLOSE -> {
                release()
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    /** Forward the runtime permission answer from [MainActivity]. */
    fun onRequestPermissionsResult(requestCode: Int, grantResults: IntArray) {
        if (requestCode != PERMISSION_REQUEST_CODE) return
        settlePendingPermissions(permissionGranted(grantResults))
    }

    /** Pure helper so the grant rule can be tested without an Android runtime. */
    fun permissionGranted(grantResults: IntArray?): Boolean =
        grantResults?.any { it == PackageManager.PERMISSION_GRANTED } == true

    private fun hasLocationPermission(context: Context): Boolean =
        ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_FINE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED ||
            ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.ACCESS_COARSE_LOCATION,
            ) == PackageManager.PERMISSION_GRANTED

    private fun requestPermission(context: Context?, result: MethodChannel.Result) {
        if (context == null) {
            result.success(false)
            return
        }
        if (hasLocationPermission(context)) {
            result.success(true)
            return
        }
        val host = hostForPermission(context)
        if (host == null) {
            result.success(false)
            return
        }
        enqueuePermissionReply { result.success(it) }
        ActivityCompat.requestPermissions(host, LOCATION_PERMISSIONS, PERMISSION_REQUEST_CODE)
    }

    @Synchronized
    private fun enqueuePermissionReply(reply: (Boolean) -> Unit) {
        pendingPermissionReplies.add(reply)
    }

    @Synchronized
    private fun takePendingPermissionReplies(): List<(Boolean) -> Unit> {
        val replies = pendingPermissionReplies.toList()
        pendingPermissionReplies.clear()
        return replies
    }

    private fun settlePendingPermissions(granted: Boolean) {
        takePendingPermissionReplies().forEach { it(granted) }
    }

    private fun hostForPermission(context: Context): Activity? = activity ?: context as? Activity

    /** Gravity-compensated acceleration above the raw motion debounce. */
    fun accelerationExceedsThreshold(dx: Float, dy: Float, dz: Float): Boolean =
        sqrt(dx * dx + dy * dy + dz * dz) > MOTION_THRESHOLD

    private fun getLocation(context: Context, result: MethodChannel.Result) {
        val fine = ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.ACCESS_FINE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED
        if (!hasLocationPermission(context)) {
            val host = hostForPermission(context)
            if (host == null) {
                result.success(null)
                return
            }
            // Defer the fix reply until the dialog is answered; close() settles
            // this queue too, so the Dart future can never hang mid-prompt.
            enqueuePermissionReply { granted ->
                if (granted) getLocation(context, result) else result.success(null)
            }
            ActivityCompat.requestPermissions(host, LOCATION_PERMISSIONS, PERMISSION_REQUEST_CODE)
            return
        }

        releaseLocation()
        val delivered = AtomicBoolean(false)
        locationDelivered = delivered
        pendingLocation = result

        val reply: (Location?) -> Unit = { fix ->
            if (delivered.compareAndSet(false, true)) {
                stopLocationRequest()
                if (fix != null) lastFix = fix
                postSuccess(
                    result,
                    fix?.let { location ->
                        mapOf(
                            "latitude" to location.latitude,
                            "longitude" to location.longitude,
                            "accuracy" to location.accuracy,
                        )
                    },
                )
            }
        }

        val cached = lastFix
        if (cached != null && isFresh(cached)) {
            reply(cached)
            return
        }

        val client = fusedClient(context)
        if (client != null && tryFused(context, client, fine, reply)) return
        requestFromManager(context, fine, reply)
    }

    /** Reflective Play-services probe; no compile-time dependency on GMS. */
    private fun fusedClient(context: Context): Any? = try {
        Class.forName("com.google.android.gms.location.LocationServices")
            .getMethod("getFusedLocationProviderClient", Context::class.java)
            .invoke(null, context)
    } catch (_: Exception) {
        null
    }

    /** Ask the fused provider for its cached fix; fall back when it is stale. */
    private fun tryFused(
        context: Context,
        client: Any,
        fine: Boolean,
        reply: (Location?) -> Unit,
    ): Boolean {
        return try {
            val task = client.javaClass.getMethod("getLastLocation").invoke(client) ?: return false
            val successClass = Class.forName("com.google.android.gms.tasks.OnSuccessListener")
            val failureClass = Class.forName("com.google.android.gms.tasks.OnFailureListener")
            val success = Proxy.newProxyInstance(
                successClass.classLoader,
                arrayOf(successClass),
            ) { _, _, args ->
                val fix = args?.firstOrNull() as? Location
                if (fix != null && isFresh(fix)) reply(fix)
                else requestFromManager(context, fine, reply)
                null
            }
            val failure = Proxy.newProxyInstance(
                failureClass.classLoader,
                arrayOf(failureClass),
            ) { _, _, _ ->
                requestFromManager(context, fine, reply)
                null
            }
            task.javaClass.getMethod("addOnSuccessListener", successClass).invoke(task, success)
            task.javaClass.getMethod("addOnFailureListener", failureClass).invoke(task, failure)
            true
        } catch (_: Exception) {
            false
        }
    }

    /** LocationManager path: newest cached fix, else one short live request. */
    private fun requestFromManager(
        context: Context,
        fine: Boolean,
        reply: (Location?) -> Unit,
    ) {
        val manager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
        if (manager == null) {
            reply(null)
            return
        }
        val providers = mutableListOf<String>()
        if (coarseOrFine(manager, LocationManager.NETWORK_PROVIDER, fine)) {
            providers.add(LocationManager.NETWORK_PROVIDER)
        }
        if (fine && coarseOrFine(manager, LocationManager.GPS_PROVIDER, fine)) {
            providers.add(LocationManager.GPS_PROVIDER)
        }
        val cached = providers.mapNotNull { safeLastFix(manager, it) }.maxByOrNull { it.time }
        if (cached != null && isFresh(cached)) {
            reply(cached)
            return
        }
        val provider = providers.firstOrNull()
        if (provider == null) {
            reply(cached)
            return
        }

        val listener = object : LocationListener {
            override fun onLocationChanged(location: Location) = reply(location)

            @Suppress("DEPRECATION")
            override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) = Unit

            override fun onProviderEnabled(provider: String) = Unit
            override fun onProviderDisabled(provider: String) = Unit
        }
        // Anything we have here is stale (fresh fixes returned above), so a
        // timed-out request reports no fix rather than pretending to be fresh.
        val timeout = Runnable { reply(null) }
        locationManager = manager
        locationListener = listener
        locationTimeout = timeout
        mainHandler.postDelayed(timeout, LOCATION_TIMEOUT_MS)
        try {
            manager.requestLocationUpdates(provider, 0L, 0f, listener, Looper.getMainLooper())
        } catch (_: Exception) {
            reply(null)
        }
    }

    private fun coarseOrFine(manager: LocationManager, provider: String, fine: Boolean): Boolean {
        // GPS needs fine permission; the network provider also works on coarse.
        if (provider == LocationManager.GPS_PROVIDER && !fine) return false
        return try {
            manager.isProviderEnabled(provider)
        } catch (_: Exception) {
            false
        }
    }

    private fun safeLastFix(manager: LocationManager, provider: String): Location? = try {
        manager.getLastKnownLocation(provider)
    } catch (_: Exception) {
        null
    }

    private fun isFresh(fix: Location): Boolean =
        fix.time > 0 && System.currentTimeMillis() - fix.time <= MAX_FIX_AGE_MS

    private fun isMoving(context: Context, result: MethodChannel.Result) {
        releaseMotion()
        val manager = context.getSystemService(Context.SENSOR_SERVICE) as? SensorManager
        val sensor = manager?.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
        if (manager == null || sensor == null) {
            result.success(true)
            return
        }

        val delivered = AtomicBoolean(false)
        motionDelivered = delivered
        pendingMotion = result
        var gravity: FloatArray? = null
        var sawEvent = false
        var listener: SensorEventListener? = null

        val finish: (Boolean) -> Unit = { moving ->
            if (delivered.compareAndSet(false, true)) {
                listener?.let { manager.unregisterListener(it) }
                motionManager = null
                stopMotionRequest()
                postSuccess(result, moving)
            }
        }
        listener = object : SensorEventListener {
            override fun onSensorChanged(event: SensorEvent) {
                if (event.sensor?.type != Sensor.TYPE_ACCELEROMETER) return
                sawEvent = true
                val values = event.values
                val previous = gravity
                if (previous == null) {
                    gravity = values.copyOf()
                    return
                }
                val dx = values[0] - previous[0]
                val dy = values[1] - previous[1]
                val dz = values[2] - previous[2]
                previous[0] = GRAVITY_ALPHA * previous[0] + (1 - GRAVITY_ALPHA) * values[0]
                previous[1] = GRAVITY_ALPHA * previous[1] + (1 - GRAVITY_ALPHA) * values[1]
                previous[2] = GRAVITY_ALPHA * previous[2] + (1 - GRAVITY_ALPHA) * values[2]
                if (accelerationExceedsThreshold(dx, dy, dz)) finish(true)
            }

            override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) = Unit
        }
        val timeout = Runnable { finish(!sawEvent) }
        motionListener = listener
        motionManager = manager
        motionTimeout = timeout
        try {
            manager.registerListener(listener, sensor, SensorManager.SENSOR_DELAY_NORMAL, mainHandler)
        } catch (_: Exception) {
            finish(true)
            return
        }
        mainHandler.postDelayed(timeout, MOTION_TIMEOUT_MS)
    }

    /** Cancel in-flight requests, replying fail-open so Dart never hangs. */
    fun release() {
        settlePendingPermissions(false)
        releaseLocation()
        releaseMotion()
    }

    private fun releaseLocation() {
        val delivered = locationDelivered ?: return
        locationDelivered = null
        if (delivered.compareAndSet(false, true)) {
            stopLocationRequest()
            pendingLocation?.let { postSuccess(it, null) }
        }
        pendingLocation = null
    }

    private fun releaseMotion() {
        val delivered = motionDelivered ?: return
        motionDelivered = null
        if (delivered.compareAndSet(false, true)) {
            motionListener?.let { listener -> motionManager?.unregisterListener(listener) }
            stopMotionRequest()
            pendingMotion?.let { postSuccess(it, true) }
        }
        pendingMotion = null
        motionManager = null
    }

    private fun stopLocationRequest() {
        locationListener?.let { listener -> locationManager?.unregisterListener(listener) }
        locationListener = null
        locationManager = null
        locationTimeout?.let { mainHandler.removeCallbacks(it) }
        locationTimeout = null
    }

    private fun stopMotionRequest() {
        motionListener = null
        motionTimeout?.let { mainHandler.removeCallbacks(it) }
        motionTimeout = null
    }

    private fun postSuccess(result: MethodChannel.Result, value: Any?) {
        // Sensor and location callbacks may land off the platform thread.
        mainHandler.post { result.success(value) }
    }
}
