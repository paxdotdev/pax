export class AnyCreatePatch {
    public idChain?: number[];
    public parentFrame?: number[];
    public zIndex?: number;

    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.zIndex = jsonMessage["z_index"];
    }

    cleanUp(){
        this.idChain = [];
        this.parentFrame = [];
        this.zIndex = -1;
    }
}

