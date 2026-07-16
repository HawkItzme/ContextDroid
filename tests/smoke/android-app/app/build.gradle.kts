plugins {
    id("com.android.application")
}

android {
    namespace = "com.example.contextdroid"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.example.contextdroid"
        minSdk = 23
        targetSdk = 37
        versionCode = 1
        versionName = "1.0"
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
