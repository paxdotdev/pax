package dev.pax;

import android.app.Instrumentation;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import android.text.Editable;
import android.text.Selection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.EditText;
import android.widget.TextView;
import org.json.JSONObject;

/** Preserve input connections across Rust textbox changes and editable Text patches. */
public final class TextInputProbe extends Instrumentation {
    PaxActivity activity;
    EditText editor;
    Editable buffer;
    InputConnection input;
    PaxScene.Node textNode;
    TextView textEditor;
    Editable textBuffer;
    InputConnection textInput;
    interface Check {void run() throws Exception;}
    void ui(Check check) throws Exception {
        Throwable[] error={null};
        runOnMainSync(()-> {try {check.run();}catch(Throwable failure){error[0]=failure;}});
        if(error[0]!=null)throw new AssertionError(error[0]);
    }
    void await(Check check) throws Exception {
        long deadline=SystemClock.uptimeMillis()+30000;Throwable last=null;
        do {try{ui(check);return;}catch(Throwable error){last=error;SystemClock.sleep(50);}}while(SystemClock.uptimeMillis()<deadline);
        throw new AssertionError("Text input condition timed out",last);
    }
    void transformed(String expected) throws Exception {
        await(()-> {
            require(!activity.failed,"host failed");
            require(editor.getText().toString().equals(expected),"Rust transformation: "+editor.getText());
        });
        ui(()-> {
            require(editor.getText()==buffer,"application update replaced the IME's editable buffer");
            require(editor.getSelectionStart()==expected.length() && editor.getSelectionEnd()==expected.length(),"selection after transformed input");
        });
    }
    @Override public void onCreate(Bundle args){super.onCreate(args);start();}
    @Override public void onStart() {
        Bundle result=new Bundle();int checks=0;
        try {
            activity=(PaxActivity)startActivitySync(new Intent().setClassName(getTargetContext(),"dev.pax.PaxActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("pax.startup","uppercase"));
            await(()-> {
                require(activity.scene!=null && activity.session.activated,"first frame");
                for(PaxScene.Node node:activity.scene.nodes.values())if(node.control instanceof EditText)editor=(EditText)node.control;
                require(editor!=null,"native textbox");
            });
            ui(()-> {
                editor.requestFocus();editor.setSelection(editor.length());buffer=editor.getText();
                input=editor.onCreateInputConnection(new EditorInfo());require(input!=null,"native input connection");
                require(input.commitText("-a",1),"first input commit");
            });
            transformed("NATIVE TEXT AND IME-A");checks++;
            ui(()->require(input.commitText("pp",1),"same connection accepts more input"));
            transformed("NATIVE TEXT AND IME-APP");checks++;
            ui(()-> {
                require(input.setSelection(editor.length()-3,editor.length()),"select replacement range");
                require(input.commitText("z",1),"replace selected range");
            });
            transformed("NATIVE TEXT AND IME-Z");checks++;
            activity=(PaxActivity)startActivitySync(new Intent().setClassName(getTargetContext(),"dev.pax.PaxActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK|Intent.FLAG_ACTIVITY_CLEAR_TASK).putExtra("pax.startup","rich-text"));
            await(()-> {
                require(activity.scene!=null && activity.session.activated,"editable Text first frame");
                for(PaxScene.Node node:activity.scene.nodes.values())if(Boolean.TRUE.equals(node.editableText))textNode=node;
                require(textNode!=null,"native editable Text");
            });
            ui(()-> {
                textEditor=(TextView)textNode.control;textEditor.requestFocus();
                textBuffer=(Editable)textEditor.getText();Selection.setSelection(textBuffer,textBuffer.length());
                textInput=textEditor.onCreateInputConnection(new EditorInfo());require(textInput!=null,"editable Text input connection");
                // Exercise the host's application-update boundary while keeping
                // the same native connection. The next commit must still reach
                // the displayed buffer and the Rust-bound property.
                JSONObject patch=new JSONObject(textNode.model.toString());
                patch.put("content","NATIVE TEXT AND IME");textNode.apply(patch);
                require(textEditor.getText()==textBuffer,"editable Text application update replaced its buffer");
                require(textInput.commitText("-continued",1),"editable Text connection accepts more input");
            });
            await(()-> {
                require(!activity.failed,"editable Text host failed");
                require(textEditor.getText().toString().equals("NATIVE TEXT AND IME-continued"),"editable Text lost continued input");
                boolean echoed=false;
                for(PaxScene.Node node:activity.scene.nodes.values())if(node.control instanceof TextView &&
                    ((TextView)node.control).getText().toString().equals("Text edit: NATIVE TEXT AND IME-continued"))echoed=true;
                require(echoed,"editable Text did not write through to its Rust-bound property");
                require(textEditor.getText()==textBuffer,"editable Text echo replaced its buffer");
            });checks++;
            result.putString("passed","true");result.putInt("checks",checks);
        }catch(Throwable error){result.putString("passed","false");result.putString("error",android.util.Log.getStackTraceString(error));}
        finish("true".equals(result.getString("passed"))?-1:0,result);
    }
    static void require(boolean condition,String message){if(!condition)throw new AssertionError(message);}
}
