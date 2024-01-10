export class OcclusionUpdatePatch {
    public idChain?: number[];
    public zIndex?: number;
    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];

        this.zIndex = jsonMessage["z_index"];
    }
    cleanUp(){
        this.idChain = [];
        this.zIndex = -1;
    }
}

