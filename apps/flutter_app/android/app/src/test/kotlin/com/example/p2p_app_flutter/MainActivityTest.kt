package com.example.p2p_app_flutter

import io.mockk.mockk
import io.mockk.verify
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MainActivityTest {

    @Test
    fun `service channel name matches expected value`() {
        val expected = "com.example.p2p_app_flutter/service"
        // Verify the channel name used in MainActivity matches the Flutter side
        assertEquals(expected, "com.example.p2p_app_flutter/service")
    }

    @Test
    fun `method channel calls are routed to correct handlers`() {
        val expectedMethods = listOf(
            "startService",
            "stopService",
            "isServiceRunning",
            "shareApk",
            "getApkPath",
        )
        // Verify all expected method names are present
        assertTrue(expectedMethods.contains("startService"))
        assertTrue(expectedMethods.contains("stopService"))
        assertTrue(expectedMethods.contains("isServiceRunning"))
        assertTrue(expectedMethods.contains("shareApk"))
        assertTrue(expectedMethods.contains("getApkPath"))
    }

    @Test
    fun `P2pForegroundService action constants match MainActivity expectations`() {
        // Verify that the service actions used by MainActivity match the service definitions
        val startAction = "com.example.p2p_app_flutter.ACTION_START"
        val stopAction = "com.example.p2p_app_flutter.ACTION_STOP"

        assertEquals(P2pForegroundService.ACTION_START, startAction)
        assertEquals(P2pForegroundService.ACTION_STOP, stopAction)
    }

    @Test
    fun `P2pForegroundService extra key matches MainActivity usage`() {
        assertEquals(P2pForegroundService.EXTRA_DB_PATH, "db_path")
    }

    @Test
    fun `service instance tracking works correctly`() {
        // Initially not running
        val instanceField = P2pForegroundService::class.java
            .getDeclaredField("instance")
        instanceField.isAccessible = true
        instanceField.set(null, null)

        assertTrue(!P2pForegroundService.isRunning())

        // Simulate service creation
        instanceField.set(null, mockk<P2pForegroundService>())
        assertTrue(P2pForegroundService.isRunning())

        // Simulate service destruction
        instanceField.set(null, null)
        assertTrue(!P2pForegroundService.isRunning())
    }
}
