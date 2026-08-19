package com.example.p2p_app_flutter

import io.mockk.every
import io.mockk.mockk
import io.mockk.mockkStatic
import io.mockk.unmockkStatic
import io.mockk.verify
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class P2pForegroundServiceTest {

    @Before
    fun setUp() {
        // Reset the static instance before each test via reflection
        val instanceField = P2pForegroundService::class.java
            .getDeclaredField("instance")
        instanceField.isAccessible = true
        instanceField.set(null, null)

        val channelField = P2pForegroundService::class.java
            .getDeclaredField("methodChannel")
        channelField.isAccessible = true
        channelField.set(null, null)
    }

    @After
    fun tearDown() {
        val instanceField = P2pForegroundService::class.java
            .getDeclaredField("instance")
        instanceField.isAccessible = true
        instanceField.set(null, null)

        val channelField = P2pForegroundService::class.java
            .getDeclaredField("methodChannel")
        channelField.isAccessible = true
        channelField.set(null, null)
    }

    @Test
    fun `isRunning returns false when no instance exists`() {
        assertFalse(P2pForegroundService.isRunning())
    }

    @Test
    fun `isRunning returns true when instance is set`() {
        val instanceField = P2pForegroundService::class.java
            .getDeclaredField("instance")
        instanceField.isAccessible = true
        instanceField.set(null, mockk<P2pForegroundService>())

        assertTrue(P2pForegroundService.isRunning())
    }

    @Test
    fun `setMethodChannel stores the channel`() {
        val channel = mockk<io.flutter.plugin.common.MethodChannel>(relaxed = true)
        P2pForegroundService.setMethodChannel(channel)

        val channelField = P2pForegroundService::class.java
            .getDeclaredField("methodChannel")
        channelField.isAccessible = true
        val stored = channelField.get(null) as? io.flutter.plugin.common.MethodChannel

        assertNotNull(stored)
        assertTrue(stored === channel)
    }

    @Test
    fun `companion constants have expected values`() {
        assertEquals("p2p_networking", P2pForegroundService.CHANNEL_ID)
        assertEquals(1, P2pForegroundService.NOTIFICATION_ID)
        assertEquals(
            "com.example.p2p_app_flutter.ACTION_START",
            P2pForegroundService.ACTION_START
        )
        assertEquals(
            "com.example.p2p_app_flutter.ACTION_STOP",
            P2pForegroundService.ACTION_STOP
        )
        assertEquals("db_path", P2pForegroundService.EXTRA_DB_PATH)
    }

    @Test
    fun `onBind returns null`() {
        val service = P2pForegroundService()
        val result = service.onBind(null)
        assertNull(result)
    }
}

private fun assertEquals(expected: Any?, actual: Any?) {
    org.junit.Assert.assertEquals(expected, actual)
}
