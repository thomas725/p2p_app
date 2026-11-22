package com.example.p2p_app_flutter

import io.mockk.every
import io.mockk.mockk
import io.mockk.verify
import org.junit.Assert.assertEquals
import org.junit.Test

class CallAudioTest {

    @Test
    fun `channel and method names match the Dart side`() {
        // US-044: drift here silently drops every call frame, so pin the whole
        // method-channel contract shared with lib/src/util/call_audio_android.dart.
        assertEquals("com.example.p2p_app_flutter/call_audio", CallAudio.CHANNEL)
        assertEquals(
            "com.example.p2p_app_flutter/call_audio_events",
            CallAudio.EVENTS,
        )
        assertEquals("start", CallAudio.START)
        assertEquals("play", CallAudio.PLAY)
        assertEquals("stop", CallAudio.STOP)
        assertEquals("pcm", CallAudio.PCM)
    }

    @Test
    fun `frame geometry matches the Rust media wire format`() {
        assertEquals(16000, CallAudio.SAMPLE_RATE)
        assertEquals(320, CallAudio.FRAME_SAMPLES)
        assertEquals(640, CallAudio.FRAME_BYTES)
    }

    @Test
    fun `unknown methods reply notImplemented so Dart never hangs`() {
        val result = mockk<MethodChannel.Result>(relaxed = true)
        val call = mockk<MethodCall>()
        every { call.method } returns "bogus"

        CallAudio.handle(call, result)

        verify { result.notImplemented() }
    }

    @Test
    fun `a permission result with no pending start is a no-op`() {
        // Hang-up closes the prompt: stop() clears the pending result, so a
        // late grant must not open the microphone after the call is over.
        CallAudio.onRequestPermissionsResult(CallAudio.PERMISSION_REQUEST_CODE, intArrayOf(1))
        CallAudio.onRequestPermissionsResult(9999, intArrayOf(1))
    }
}
