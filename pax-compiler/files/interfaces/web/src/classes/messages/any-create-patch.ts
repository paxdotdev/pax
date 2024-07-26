export class AnyCreatePatch {
    public id?: number;
    public parentFrame?: number;
    public occlusionLayerId?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.occlusionLayerId = jsonMessage["occlusion_layer_id"];
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.occlusionLayerId = -1;
    }
}

