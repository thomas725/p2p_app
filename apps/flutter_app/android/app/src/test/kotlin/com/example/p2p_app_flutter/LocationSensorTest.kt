package com.example.p2p_app_flutter

import android.content.pm.PackageManager
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.mockk.every
import io.mockk.mockk
import io.mockk.verify
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LocationSensorTest {

    @Test
    fun `channel and method names match the Dart side`() {
        // Drift here silently drops every fix, so pin the whole contract shared
        // with lib/src/util/location_device_io.dart.
        assertEquals("p2p_app/location", LocationSensor.CHANNEL)
        assertEquals("requestPermission", LocationSensor.REQUEST_PERMISSION)
        assertEquals("getLocation", LocationSensor.GET_LOCATION)
        assertEquals("isMoving", LocationSensor.IS_MOVING)
        assertEquals("close", LocationSensor.CLOSE)
    }

    @Test
    fun `unknown methods reply notImplemented so Dart never hangs`() {
        val result = mockk<MethodChannel.Result>(relaxed = true)
        val call = mockk<MethodCall>()
        every { call.method } returns "bogus"

        LocationSensor.handle(call, null, result)

        verify { result.notImplemented() }
    }

    @Test
    fun `no attached context fails open for both methods`() {
        val result = mockk<MethodChannel.Result>(relaxed = true)
        val call = mockk<MethodCall>()

        every { call.method } returns LocationSensor.GET_LOCATION
        LocationSensor.handle(call, null, result)
        verify { result.success(null) }

        every { call.method } returns LocationSensor.IS_MOVING
        LocationSensor.handle(call, null, result)
        verify { result.success(true) }

        every { call.method } returns LocationSensor.REQUEST_PERMISSION
        LocationSensor.handle(call, null, result)
        verify { result.success(false) }
    }

    @Test
    fun `a coarse or fine grant is accepted and an empty answer is denied`() {
        assertTrue(
            LocationSensor.permissionGranted(
                intArrayOf(PackageManager.PERMISSION_GRANTED),
            ),
        )
        assertTrue(
            LocationSensor.permissionGranted(
                intArrayOf(
                    PackageManager.PERMISSION_DENIED,
                    PackageManager.PERMISSION_GRANTED,
                ),
            ),
        )
        assertFalse(LocationSensor.permissionGranted(intArrayOf()))
        assertFalse(
            LocationSensor.permissionGranted(
                intArrayOf(PackageManager.PERMISSION_DENIED),
            ),
        )
        assertFalse(LocationSensor.permissionGranted(null))
    }

    @Test
    fun `permission answers with no pending request are safe`() {
        LocationSensor.onRequestPermissionsResult(LocationSensor.PERMISSION_REQUEST_CODE, intArrayOf())
        LocationSensor.onRequestPermissionsResult(9999, intArrayOf(PackageManager.PERMISSION_GRANTED))
    }

    @Test
    fun `close with no pending request is safe`() {
        val result = mockk<MethodChannel.Result>(relaxed = true)
        val call = mockk<MethodCall>()
        every { call.method } returns LocationSensor.CLOSE

        LocationSensor.handle(call, null, result)
        LocationSensor.handle(call, null, result)

        verify(exactly = 2) { result.success(null) }
    }

    @Test
    fun `motion threshold is a raw acceleration debounce`() {
        assertFalse(LocationSensor.accelerationExceedsThreshold(0.1f, 0f, 0f))
        assertFalse(LocationSensor.accelerationExceedsThreshold(0.3f, -0.2f, 0.1f))
        assertTrue(LocationSensor.accelerationExceedsThreshold(0.6f, 0f, 0f))
        assertTrue(LocationSensor.accelerationExceedsThreshold(-0.3f, 0.4f, 0f))
    }
}
