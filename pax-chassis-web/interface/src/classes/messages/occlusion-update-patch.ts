export class OcclusionUpdatePatch {
    public id?: number;
    public occlusionLayerId?: number;
    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
    }
    cleanUp(){
        this.id = undefined;
        this.occlusionLayerId = -1;
    }
}

