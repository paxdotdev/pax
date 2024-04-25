export class AnyCreatePatch {
    public idChain?: number[];
    public parentFrame?: number;
    public occlusionLayerId?: number;

    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.occlusionLayerId = jsonMessage["z_index"];
    }

    cleanUp(){
        this.idChain = [];
        this.parentFrame = undefined;
        this.occlusionLayerId = -1;
    }
}

