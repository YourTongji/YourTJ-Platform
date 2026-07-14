package de.yourtj.mobile

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean

class MainActivity : FlutterActivity() {
    private val mainHandler = Handler(Looper.getMainLooper())
    private val exportWriter = Executors.newSingleThreadExecutor { runnable ->
        Thread(runnable, "account-export-writer")
    }
    private var pendingExport: PendingExport? = null
    private var activityDestroyed = false

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            ACCOUNT_EXPORT_CHANNEL,
        ).setMethodCallHandler { call, result ->
            when (call.method) {
                SAVE_ACCOUNT_EXPORT_METHOD -> saveAccountExport(
                    fileName = call.argument<String>("fileName"),
                    bytes = call.argument<ByteArray>("bytes"),
                    result = result,
                )
                CANCEL_ACCOUNT_EXPORT_METHOD -> cancelPendingExport(result)
                else -> result.notImplemented()
            }
        }
    }

    private fun saveAccountExport(
        fileName: String?,
        bytes: ByteArray?,
        result: MethodChannel.Result,
    ) {
        if (pendingExport != null) {
            bytes?.fill(0)
            result.error("EXPORT_BUSY", "An account export is already pending.", null)
            return
        }
        if (
            fileName != ACCOUNT_EXPORT_FILE_NAME ||
                bytes == null ||
                bytes.isEmpty() ||
                bytes.size > MAX_EXPORT_BYTES
        ) {
            bytes?.fill(0)
            result.error("INVALID_ARGUMENT", "The account export payload is invalid.", null)
            return
        }

        val pending = PendingExport(bytes.copyOf(), result)
        bytes.fill(0)
        pendingExport = pending
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
            type = "application/json"
            putExtra(Intent.EXTRA_TITLE, ACCOUNT_EXPORT_FILE_NAME)
        }
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, ACCOUNT_EXPORT_REQUEST_CODE)
        } catch (_: ActivityNotFoundException) {
            finishWithError(
                pending,
                code = "EXPORT_UNAVAILABLE",
                message = "No system document provider is available.",
            )
        } catch (_: SecurityException) {
            finishWithError(
                pending,
                code = "EXPORT_UNAVAILABLE",
                message = "The system document provider is unavailable.",
            )
        }
    }

    private fun cancelPendingExport(result: MethodChannel.Result) {
        val pending = pendingExport
        if (pending == null) {
            result.success(false)
            return
        }
        pending.cancelled.set(true)
        if (!pending.isWriting) {
            try {
                finishActivity(ACCOUNT_EXPORT_REQUEST_CODE)
            } catch (_: Exception) {
                // There is no app-owned file to clean before the picker returns a destination.
            }
            finishWithStatus(pending, "cancelled")
        }
        result.success(true)
    }

    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != ACCOUNT_EXPORT_REQUEST_CODE) {
            super.onActivityResult(requestCode, resultCode, data)
            return
        }
        val pending = pendingExport ?: return
        if (resultCode != Activity.RESULT_OK || pending.cancelled.get()) {
            finishWithStatus(pending, "cancelled")
            return
        }
        val destination = data?.data
        if (destination == null) {
            finishWithError(
                pending,
                code = "EXPORT_WRITE_FAILED",
                message = "The system did not return a writable destination.",
            )
            return
        }

        pending.isWriting = true
        try {
            exportWriter.execute { writeExport(pending, destination) }
        } catch (_: RejectedExecutionException) {
            pending.isWriting = false
            finishWithError(
                pending,
                code = "EXPORT_INTERRUPTED",
                message = "The account export writer is no longer available.",
            )
        }
    }

    private fun writeExport(pending: PendingExport, destination: Uri) {
        var didWrite = false
        try {
            if (!pending.cancelled.get()) {
                val output = contentResolver.openOutputStream(destination, "wt")
                if (output != null) {
                    output.use { stream ->
                        var offset = 0
                        while (offset < pending.bytes.size && !pending.cancelled.get()) {
                            val count = minOf(WRITE_CHUNK_BYTES, pending.bytes.size - offset)
                            stream.write(pending.bytes, offset, count)
                            offset += count
                        }
                        if (!pending.cancelled.get()) {
                            stream.flush()
                            didWrite = true
                        }
                    }
                }
            }
        } catch (_: Exception) {
            didWrite = false
        }

        val deletedAfterFailure = if (didWrite) {
            false
        } else {
            deleteDestination(destination)
        }
        mainHandler.post {
            pending.isWriting = false
            completeWrite(
                pending = pending,
                destination = destination,
                didWrite = didWrite,
                deletedAfterFailure = deletedAfterFailure,
            )
        }
    }

    private fun completeWrite(
        pending: PendingExport,
        destination: Uri,
        didWrite: Boolean,
        deletedAfterFailure: Boolean,
    ) {
        if (pendingExport !== pending) {
            pending.bytes.fill(0)
            shutdownWriterIfDestroyed()
            return
        }
        if (pending.cancelled.get()) {
            if (!didWrite) {
                if (deletedAfterFailure) {
                    finishWithStatus(pending, "cancelled")
                } else {
                    finishWithError(
                        pending,
                        code = "EXPORT_WRITE_FAILED",
                        message = "A cancelled destination could not be removed.",
                    )
                }
                return
            }
            deleteCompletedExportAfterCancellation(pending, destination)
            return
        }
        if (didWrite) {
            finishWithStatus(pending, "saved")
        } else {
            finishWithError(
                pending,
                code = "EXPORT_WRITE_FAILED",
                message = "The selected destination could not be written.",
            )
        }
    }

    private fun deleteCompletedExportAfterCancellation(
        pending: PendingExport,
        destination: Uri,
    ) {
        pending.isWriting = true
        try {
            exportWriter.execute {
                val deleted = deleteDestination(destination)
                mainHandler.post {
                    pending.isWriting = false
                    if (deleted) {
                        finishWithStatus(pending, "cancelled")
                    } else {
                        finishWithError(
                            pending,
                            code = "EXPORT_WRITE_FAILED",
                            message = "A cancelled destination could not be removed.",
                        )
                    }
                }
            }
        } catch (_: RejectedExecutionException) {
            pending.isWriting = false
            finishWithError(
                pending,
                code = "EXPORT_INTERRUPTED",
                message = "The cancelled destination could not be removed.",
            )
        }
    }

    private fun deleteDestination(destination: Uri): Boolean {
        return try {
            contentResolver.delete(destination, null, null) > 0
        } catch (_: Exception) {
            false
        }
    }

    private fun finishWithStatus(pending: PendingExport, status: String) {
        if (pendingExport !== pending) {
            pending.bytes.fill(0)
            shutdownWriterIfDestroyed()
            return
        }
        pendingExport = null
        pending.bytes.fill(0)
        pending.result.success(status)
        shutdownWriterIfDestroyed()
    }

    private fun finishWithError(pending: PendingExport, code: String, message: String) {
        if (pendingExport !== pending) {
            pending.bytes.fill(0)
            shutdownWriterIfDestroyed()
            return
        }
        pendingExport = null
        pending.bytes.fill(0)
        pending.result.error(code, message, null)
        shutdownWriterIfDestroyed()
    }

    override fun onDestroy() {
        activityDestroyed = true
        val pending = pendingExport
        if (pending == null) {
            exportWriter.shutdown()
        } else {
            pending.cancelled.set(true)
            if (!pending.isWriting) {
                try {
                    finishActivity(ACCOUNT_EXPORT_REQUEST_CODE)
                } catch (_: Exception) {
                    // The activity is already ending and owns no destination URI at this stage.
                }
                finishWithError(
                    pending,
                    code = "EXPORT_INTERRUPTED",
                    message = "The activity ended before the export was saved.",
                )
            }
        }
        super.onDestroy()
    }

    private fun shutdownWriterIfDestroyed() {
        if (activityDestroyed && pendingExport == null) {
            exportWriter.shutdown()
        }
    }

    private class PendingExport(
        val bytes: ByteArray,
        val result: MethodChannel.Result,
    ) {
        val cancelled = AtomicBoolean(false)
        var isWriting = false
    }

    private companion object {
        const val ACCOUNT_EXPORT_CHANNEL = "de.yourtj.mobile/account-export"
        const val SAVE_ACCOUNT_EXPORT_METHOD = "saveAccountExport"
        const val CANCEL_ACCOUNT_EXPORT_METHOD = "cancelAccountExport"
        const val ACCOUNT_EXPORT_FILE_NAME = "yourtj-account-export.json"
        const val MAX_EXPORT_BYTES = 16 * 1024 * 1024
        const val WRITE_CHUNK_BYTES = 64 * 1024
        const val ACCOUNT_EXPORT_REQUEST_CODE = 0x594A
    }
}
