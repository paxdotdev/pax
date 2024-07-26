export class OcclusionUpdatePatch {
    public id?: number;
    public occlusionLayerId?: number;
    public zIndex?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
        this.zIndex = jsonMessage["z_index"];
    }
    cleanUp(){
        this.id = undefined;
        this.occlusionLayerId = -1;
        this.zIndex = -1;
    }
}

