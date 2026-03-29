package cicbyte.byte_memos_mobile

import android.content.Intent
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val channelName = "byte_memos/share"

    /// 分享进来的文字（冷启动时 Flutter 尚未就绪，先暂存）
    private var pendingShareText: String? = null
    private var channel: MethodChannel? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).also { ch ->
            // 冷启动的分享内容由 Dart 侧就绪后调 takePendingShare 拉取
            ch.setMethodCallHandler { call, result ->
                when (call.method) {
                    "takePendingShare" -> {
                        result.success(pendingShareText)
                        pendingShareText = null
                    }
                    else -> result.notImplemented()
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        // 热路径：app 已在后台运行时接收分享
        intent.takeShareText()?.let(sendShareText)
    }

    private val sendShareText: (String) -> Unit = { text ->
        channel?.invokeMethod("shareText", text)
    }

    /// 冷启动路径：onCreate 时 Flutter engine 还没起来，暂存等 Dart 来取
    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        pendingShareText = intent?.takeShareText() ?: pendingShareText
    }
}

/// 从 ACTION_SEND intent 提取 EXTRA_TEXT（仅 text/plain）
private fun Intent.takeShareText(): String? =
    if (action == Intent.ACTION_SEND && type == "text/plain") {
        getStringExtra(Intent.EXTRA_TEXT)
    } else {
        null
    }
