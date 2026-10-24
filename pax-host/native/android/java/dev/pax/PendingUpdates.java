package dev.pax;

import org.json.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** Latest native state for one uncommitted frame, retaining lifecycle/event order. */
final class PendingUpdates {
    private final JSONArray messages=new JSONArray();
    private final HashMap<String,JSONObject> updates=new HashMap<>();
    private int bytes;

    int bytes() {return bytes;}
    JSONArray messages() {return messages;}
    void clear() {while(messages.length()>0) messages.remove(messages.length()-1);updates.clear();bytes=0;}

    void add(JSONObject message) throws JSONException {
        String kind=message.keys().next();
        Object value=message.get(kind);
        if(kind.endsWith("Update") && value instanceof JSONObject && ((JSONObject)value).has("id")) {
            JSONObject patch=(JSONObject)value;
            String key=kind+":"+patch.getInt("id");
            JSONObject previous=updates.get(key);
            if(previous!=null) {
                bytes-=size(previous);
                merge(previous,patch);
                bytes+=size(previous);
            } else {
                updates.put(key,patch);messages.put(message);bytes+=size(message);
            }
        } else {
            if(kind.endsWith("Create") || kind.endsWith("Delete")) {
                int id=value instanceof JSONObject?((JSONObject)value).getInt("id"):((Number)value).intValue();
                // An id can be reused after deletion. Never merge a replacement
                // node's patch into the previous node's lifetime.
                updates.keySet().removeIf(key -> key.endsWith(":"+id));
            }
            messages.put(message);bytes+=size(message);
        }
        if(bytes>16*1024*1024) throw new IllegalStateException("Android pending native updates exceed 16 MiB");
    }

    void observed(String kind,int id,JSONObject fields) throws JSONException {
        JSONObject patch=updates.get(kind+"Update:"+id);
        if(patch==null) return;
        bytes-=size(patch);
        Iterator<String> keys=fields.keys();while(keys.hasNext()) patch.remove(keys.next());
        bytes+=size(patch);
    }

    private static void merge(JSONObject target,JSONObject source) throws JSONException {
        Iterator<String> keys=source.keys();
        while(keys.hasNext()) {
            String key=keys.next();
            // Null normally means an absent patch field. parent_frame alone
            // uses explicit null to move a node back into the root container.
            if(!source.isNull(key) || key.equals("parent_frame")) target.put(key,source.get(key));
        }
    }
    private static int size(JSONObject value) {return value.toString().getBytes(StandardCharsets.UTF_8).length;}
}
