package network.xchat.app;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.os.VibratorManager;
import android.webkit.JavascriptInterface;
import android.widget.Toast;

public class AndroidBridge {
    private final MainActivity activity;
    private final SharedPreferences prefs;

    public AndroidBridge(MainActivity activity) {
        this.activity = activity;
        this.prefs = activity.getSharedPreferences("xchat_prefs", Context.MODE_PRIVATE);
    }

    @JavascriptInterface
    public void toast(final String message) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Toast.makeText(activity, message, Toast.LENGTH_SHORT).show();
            }
        });
    }

    @JavascriptInterface
    public void copyToClipboard(final String text) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                ClipboardManager clipboard = (ClipboardManager) activity.getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard != null) {
                    ClipData clip = ClipData.newPlainText("xChat Address", text);
                    clipboard.setPrimaryClip(clip);
                    Toast.makeText(activity, "Address copied to clipboard!", Toast.LENGTH_SHORT).show();
                }
            }
        });
    }

    @JavascriptInterface
    public void shareAddress(final String address) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Intent sendIntent = new Intent();
                sendIntent.setAction(Intent.ACTION_SEND);
                sendIntent.putExtra(Intent.EXTRA_TEXT, "Connect with me on xChat private mesh:\n" + address);
                sendIntent.setType("text/plain");
                Intent shareIntent = Intent.createChooser(sendIntent, "Share xChat Identity");
                activity.startActivity(shareIntent);
            }
        });
    }

    @JavascriptInterface
    public void vibrate(long ms) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                VibratorManager vm = (VibratorManager) activity.getSystemService(Context.VIBRATOR_MANAGER_SERVICE);
                if (vm != null) {
                    Vibrator v = vm.getDefaultVibrator();
                    v.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE));
                }
            } else {
                Vibrator v = (Vibrator) activity.getSystemService(Context.VIBRATOR_SERVICE);
                if (v != null) {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        v.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE));
                    } else {
                        v.vibrate(ms);
                    }
                }
            }
        } catch (Exception ignored) {}
    }

    @JavascriptInterface
    public void triggerSosHaptics() {
        // Morse SOS pattern: ... --- ...
        long[] pattern = { 0, 150, 100, 150, 100, 150, 300, 400, 100, 400, 100, 400, 300, 150, 100, 150, 100, 150 };
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                VibratorManager vm = (VibratorManager) activity.getSystemService(Context.VIBRATOR_MANAGER_SERVICE);
                if (vm != null) {
                    Vibrator v = vm.getDefaultVibrator();
                    v.vibrate(VibrationEffect.createWaveform(pattern, -1));
                }
            } else {
                Vibrator v = (Vibrator) activity.getSystemService(Context.VIBRATOR_SERVICE);
                if (v != null) {
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        v.vibrate(VibrationEffect.createWaveform(pattern, -1));
                    } else {
                        v.vibrate(pattern, -1);
                    }
                }
            }
        } catch (Exception ignored) {}
    }

    @JavascriptInterface
    public void savePersistent(String key, String value) {
        prefs.edit().putString(key, value).apply();
    }

    @JavascriptInterface
    public String loadPersistent(String key) {
        return prefs.getString(key, "");
    }

    @JavascriptInterface
    public String getDeviceInfo() {
        return "{\"model\":\"" + Build.MANUFACTURER + " " + Build.MODEL + "\",\"android\":\"" + Build.VERSION.RELEASE + "\"}";
    }

    @JavascriptInterface
    public boolean isNativeAndroid() {
        return true;
    }
}
