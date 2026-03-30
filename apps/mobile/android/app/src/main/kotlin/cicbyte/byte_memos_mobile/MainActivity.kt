package cicbyte.byte_memos_mobile

import android.content.Intent
import android.net.Uri
import android.util.Base64
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val channelName = "byte_memos/share"

    /// 分享进来的文字（冷启动时 Flutter 尚未就绪，先暂存）
    private var pendingShareText: String? = null

    /// 分享进来的图片（冷启动暂存：mime + base64 内容）
    private var pendingShareImage: Map<String, Any>? = null
    private var channel: MethodChannel? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).also { ch ->
            // 冷启动的分享内容由 Dart 侧就绪后调 takePendingShare 拉取
            ch.setMethodCallHandler { call, result ->
                when (call.method) {
                    "takePendingShare" -> {
                        val payload: Any? = when {
                            pendingShareImage != null -> pendingShareImage
                            pendingShareText != null -> pendingShareText
                            else -> null
                        }
                        result.success(payload)
                        pendingShareText = null
                        pendingShareImage = null
                    }
                    else -> result.notImplemented()
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        android.util.Log.d("BmShare", "onNewIntent action=${intent.action} type=${intent.type}")
        // 热路径：app 已在后台运行时接收分享
        intent.takeShareText()?.let(sendShareText)
        intent.takeShareImage(this)?.let(sendShareImage)
    }

    private val sendShareText: (String) -> Unit = { text ->
        channel?.invokeMethod("shareText", text)
    }

    private val sendShareImage: (Map<String, Any>) -> Unit = { payload ->
        channel?.invokeMethod("shareImage", payload)
    }

    /// 冷启动路径：onCreate 时 Flutter engine 还没起来，暂存等 Dart 来取
    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        val i = intent
        pendingShareText = i?.takeShareText() ?: pendingShareText
        pendingShareImage = i?.takeShareImage(this) ?: pendingShareImage
    }
}

/// 从 ACTION_SEND intent 提取 EXTRA_TEXT（仅 text/plain）
private fun Intent.takeShareText(): String? =
    if (action == Intent.ACTION_SEND && type == "text/plain") {
        getStringExtra(Intent.EXTRA_TEXT)
    } else {
        null
    }

/// 从 ACTION_SEND intent 提取图片流（image/*，F14）
/// 读成 base64 一次性传给 Dart（附件场景量级可控，避免临时文件）
private fun Intent.takeShareImage(ctx: android.content.Context): Map<String, Any>? {
    android.util.Log.d("BmShare", "takeShareImage action=$action type=$type")
    if (action != Intent.ACTION_SEND || type?.startsWith("image/") != true) return null
    @Suppress("DEPRECATION")
    val uri: Uri = extras?.getParcelable(Intent.EXTRA_STREAM) ?: run {
        android.util.Log.d("BmShare", "EXTRA_STREAM missing: extras=${extras?.keySet()}")
        return null
    }
    android.util.Log.d("BmShare", "uri=$uri")
    return try {
        val bytes = ctx.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: run {
            android.util.Log.d("BmShare", "openInputStream null")
            return null
        }
        android.util.Log.d("BmShare", "bytes=${bytes.size}")
        // 嗅探真实类型（相册可能只给 image/*）+ 补扩展名（lastPathSegment 常是裸 id）
        val (realMime, ext) = sniff(bytes)
        var name = uri.lastPathSegment ?: "shared"
        if (!name.contains('.')) name = "$name$ext"
        mapOf(
            "mime" to realMime,
            "name" to name,
            "data" to Base64.encodeToString(bytes, Base64.NO_WRAP),
        )
    } catch (e: Exception) {
        android.util.Log.d("BmShare", "fail: $e")
        null
    }
}

/// 字节头嗅探：返回 (mime, 扩展名)，未识别回落 image/*
private fun sniff(b: ByteArray): Pair<String, String> {
    val head = b.take(12)
    val hex = head.joinToString("") { "%02x".format(it) }
    val ascii = String(b, 0, minOf(4, b.size), Charsets.US_ASCII)
    return when {
        hex.startsWith("89504e47") -> "image/png" to ".png"
        hex.startsWith("ffd8ff") -> "image/jpeg" to ".jpg"
        ascii == "GIF8" -> "image/gif" to ".gif"
        hex.startsWith("52494646") && b.size > 11 &&
            String(b, 8, 4, Charsets.US_ASCII) == "WEBP" -> "image/webp" to ".webp"
        else -> "image/*" to ".png"
    }
}
