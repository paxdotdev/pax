export class OcclusionUpdatePatch {
    public idChain?: number[];
    public occlusionLayerId?: number;
    fromPatch(jsonMessage: any) {
        this.idChain = jsonMessage["id_chain"];

        this.occlusionLayerId = jsonMessage["z_index"];
    }
    cleanUp(){
        this.idChain = [];
        this.occlusionLayerId = -1;
    }
}

