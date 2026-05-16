export class AnyCreatePatch {
    public id?: number;
    public parentFrame?: number;
    public renderLayerId?: number;

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.renderLayerId = jsonMessage["render_layer_id"];
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.renderLayerId = -1;
    }
}

