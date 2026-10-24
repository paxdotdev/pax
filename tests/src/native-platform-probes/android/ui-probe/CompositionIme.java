package dev.pax.android_ime;

import android.content.*;
import android.inputmethodservice.InputMethodService;
import android.os.Build;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.TextView;

/** Test-only IME: sends fixed composition sequences to the acceptance fixture. */
public final class CompositionIme extends InputMethodService {
    final BroadcastReceiver commands=new BroadcastReceiver() {
        @Override public void onReceive(Context context,Intent intent) {
            EditorInfo editor=getCurrentInputEditorInfo();InputConnection input=getCurrentInputConnection();
            if(editor==null || input==null || !"dev.pax.native_android_smoke".equals(editor.packageName)) {
                setResultCode(1);setResultData("No fixture input connection");return;
            }
            boolean ok;
            switch(intent.getStringExtra("step")) {
                case "ready": ok=true;break;
                case "reset":
                    input.beginBatchEdit();
                    try {ok=input.performContextMenuAction(android.R.id.selectAll) && input.commitText("",1);}
                    finally {input.endBatchEdit();}
                    break;
                case "start": ok=input.setComposingText("に",1);break;
                case "update": ok=input.setComposingText("日本",1);break;
                case "commit": ok=input.commitText("日本語",1);break;
                case "unicode": ok=input.commitText(" e\u0301 مرحبا 👩‍💻",1);break;
                case "delete-emoji": ok=input.deleteSurroundingTextInCodePoints(3,0);break;
                default: ok=false;
            }
            setResultCode(ok?-1:1);setResultData(ok?"ok":"Input operation rejected");
        }
    };
    @Override public void onCreate() {
        super.onCreate();
        IntentFilter filter=new IntentFilter("dev.pax.android_ime.STEP");
        // Only adb/system callers with DUMP may drive this test service. It
        // additionally refuses connections belonging to any other application.
        if(Build.VERSION.SDK_INT>=33) registerReceiver(commands,filter,"android.permission.DUMP",null,Context.RECEIVER_EXPORTED);
        else registerReceiver(commands,filter,"android.permission.DUMP",null);
    }
    @Override public View onCreateInputView() {
        TextView view=new TextView(this);view.setText("Pax input qualification");view.setPadding(16,16,16,16);return view;
    }
    @Override public boolean onEvaluateFullscreenMode() {return false;}
    @Override public void onDestroy() {unregisterReceiver(commands);super.onDestroy();}
}
