package cicbyte.byte_memos_mobile

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.RemoteViews

/// 快记小组件（F16）：整块点击发 bytememos://quicknote 深链
class QuickNoteWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse("bytememos://quicknote")).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val pi = PendingIntent.getActivity(context, 0, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        for (id in appWidgetIds) {
            val views = RemoteViews(context.packageName, R.layout.quick_note_widget)
            views.setOnClickPendingIntent(R.id.widget_root, pi)
            views.setOnClickPendingIntent(R.id.widget_btn, pi)
            appWidgetManager.updateAppWidget(id, views)
        }
    }
}
