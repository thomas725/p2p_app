import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("kotlin-android")
    id("jacoco")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

android {
    namespace = "com.example.p2p_app_flutter"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = "29.0.14206865"

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "com.example.p2p_app_flutter"
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    // Prevent packaging stale .so files from previous builds
    packaging {
        jniLibs {
            pickFirsts += setOf("**/*.so")
        }
    }
}

flutter {
    source = "../.."
}

tasks.withType<Test>().configureEach {
    extensions.configure(JacocoTaskExtension::class) {
        isEnabled = true
    }
}

tasks.register<JacocoReport>("testDebugUnitTestJacocoReport") {
    dependsOn("testDebugUnitTest")

    reports {
        xml.required.set(true)
        xml.outputLocation.set(file("${layout.buildDirectory.get().asFile.parentFile}/reports/jacoco/testDebugUnitTest/jacocoTestReport.xml"))
        html.required.set(false)
    }

    classDirectories.setFrom(
        fileTree("../../build/app/tmp/kotlin-classes/debug") {
            exclude("**/R\$*.class")
            exclude("**/BuildConfig.*")
        }
    )

    sourceDirectories.setFrom(files("src/main/kotlin"))

    executionData.setFrom(
        fileTree("build") {
            include("**/*.exec")
        }.plus(
            fileTree("../../build/app/jacoco") {
                include("**/*.exec")
            }
        )
    )
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("io.mockk:mockk:1.14.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.10.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}
