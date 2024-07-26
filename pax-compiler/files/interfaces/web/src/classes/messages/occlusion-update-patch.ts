export class OcclusionUpdatePatch {
    public id?: number;
    public occlusionLayerId?: number;
    public zIndex?: number;
    public parentFrame?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
        this.zIndex = jsonMessage["z_index"];
        this.parentFrame = jsonMessage["parent_frame"];
    }
    cleanUp(){
        this.id = undefined;
        this.occlusionLayerId = -1;
        this.zIndex = -1;
        this.parentFrame = undefined;
    }
}

