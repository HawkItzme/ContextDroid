package com.example.contextdroid

import org.junit.Assert.assertEquals
import org.junit.Test

class FailingTest {
    @Test fun fails() = assertEquals("expected", "actual")
}
